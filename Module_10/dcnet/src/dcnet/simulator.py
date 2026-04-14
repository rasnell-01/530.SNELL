from __future__ import annotations

from dataclasses import dataclass, field

from .config import SimConfig
from .metrics import SimulationReport, build_report
from .model import Flow, Packet
from .routing import RoutePlanner
from .topology import Topology


@dataclass(slots=True)
class Simulator:
    topology: Topology
    cfg: SimConfig
    flows: list[Flow]
    route_planner: RoutePlanner = field(init=False)
    current_tick: int = 0
    next_packet_id: int = 0
    total_packets_created: int = 0

    def __post_init__(self) -> None:
        self.route_planner = RoutePlanner(
            self.topology,
            use_flow_id_in_hash=self.cfg.ecmp_use_flow_id,
        )

    # ------------------------------------------------------------------
    # Main loop
    # ------------------------------------------------------------------

    def run(self) -> SimulationReport:
        for tick in range(self.cfg.ticks):
            self.current_tick = tick
            self._record_queue_samples()
            self._inject_new_packets()
            self._service_links()
            self._update_completed_flows()
        return build_report(
            self.flows,
            list(self.topology.links.values()),
            self.total_packets_created,
            spine_switch_ids=self.topology.spine_switches,
        )

    # ------------------------------------------------------------------
    # Per-tick helpers
    # ------------------------------------------------------------------

    def _record_queue_samples(self) -> None:
        for link in self.topology.links.values():
            link.record_queue_sample()

    def _inject_new_packets(self) -> None:
        """Inject up to cfg.inject_rate_packets_per_tick new packets per flow.

        Each packet is immediately placed on the first-hop link queue.  If the
        queue is full the packet is dropped and the flow's drop counter is
        incremented so we can measure per-flow loss.
        """
        for flow in self.flows:
            if flow.start_tick > self.current_tick:
                continue
            if flow.packets_created >= flow.size_packets:
                continue

            # Inject up to the configured injection rate per flow per tick.
            to_inject = min(
                self.cfg.inject_rate_packets_per_tick,
                flow.size_packets - flow.packets_created,
            )

            for _ in range(to_inject):
                packet = Packet(
                    packet_id=self.next_packet_id,
                    flow_id=flow.flow_id,
                    src=flow.src,
                    dst=flow.dst,
                    created_tick=self.current_tick,
                    current_node=flow.src,
                )
                self.next_packet_id += 1
                self.total_packets_created += 1
                flow.packets_created += 1

                next_hop = self.route_planner.next_hop(
                    packet.current_node, packet.src, packet.dst, packet.flow_id
                )
                if next_hop is None:
                    # Newly-created packet at src should always have a next hop.
                    continue

                link = self.topology.links[(packet.current_node, next_hop)]
                if not link.enqueue(packet):
                    # Queue full: packet dropped at the source uplink.
                    flow.packets_dropped += 1

    def _service_links(self) -> None:
        """Two-phase transmission: drain all queues first, then forward arrivals.

        Phase 1 – collect: remove up to capacity_packets_per_tick packets from
        every link queue into a temporary ``arrivals`` list.  This snapshot
        ensures no packet can be forwarded AND re-transmitted in the same tick.

        Phase 2 – forward: for each arrived packet, either mark it delivered
        (packet.dst reached) or enqueue it on the next-hop link.  Packets
        forwarded in phase 2 sit in a queue that was already drained this tick,
        so they will not move again until the next tick.
        """
        arrivals: list[tuple[Packet, str]] = []

        # ---- Phase 1: drain all link queues ----
        for link in self.topology.links.values():
            packets_to_send = min(link.capacity_packets_per_tick, len(link.queue))
            for _ in range(packets_to_send):
                packet = link.queue.popleft()
                link.transmitted_packets += 1
                packet.current_node = link.dst
                arrivals.append((packet, link.dst))

        # ---- Phase 2: forward or deliver ----
        for packet, node_id in arrivals:
            if node_id == packet.dst:
                # Final delivery
                packet.delivered_tick = self.current_tick
                flow = self.flows[packet.flow_id]
                flow.packets_delivered += 1
                continue

            next_hop = self.route_planner.next_hop(
                node_id, packet.src, packet.dst, packet.flow_id
            )
            if next_hop is None:
                continue

            next_link = self.topology.links[(node_id, next_hop)]
            if not next_link.enqueue(packet):
                # Queue full mid-path: track the drop against the owning flow.
                flow = self.flows[packet.flow_id]
                flow.packets_dropped += 1

    def _update_completed_flows(self) -> None:
        for flow in self.flows:
            if flow.completion_tick is None and flow.is_complete:
                flow.completion_tick = self.current_tick

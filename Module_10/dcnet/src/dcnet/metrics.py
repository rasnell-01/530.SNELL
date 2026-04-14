from __future__ import annotations

from dataclasses import dataclass, field

from .model import Flow, Link


@dataclass(slots=True)
class LinkStats:
    link_id: str                  # "src->dst"
    transmitted: int
    dropped: int
    utilization: float            # fraction of ticks the queue was non-empty
    avg_queue_length: float
    max_queue_length: int


@dataclass(slots=True)
class SimulationReport:
    # Packet-level counters
    total_packets_created: int
    total_packets_delivered: int
    total_packets_dropped: int

    # Flow-level counters
    completed_flows: int
    total_flows: int

    # Flow completion time (ticks from flow.start_tick to completion)
    average_flow_completion_time: float
    max_flow_completion_time: int
    p99_flow_completion_time: int

    # Queue stats aggregated across all links
    average_queue_length: float
    max_queue_length: int

    # Per-link stats (sorted by utilization descending)
    link_stats: list[LinkStats] = field(default_factory=list)

    # Spine utilization breakdown (for ECMP analysis)
    spine_utilization: dict[str, float] = field(default_factory=dict)

    def delivery_rate(self) -> float:
        if self.total_packets_created == 0:
            return 0.0
        return self.total_packets_delivered / self.total_packets_created

    def drop_rate(self) -> float:
        if self.total_packets_created == 0:
            return 0.0
        return self.total_packets_dropped / self.total_packets_created

    def print_summary(self, workload_name: str = "") -> None:
        header = f"=== Simulation Report"
        if workload_name:
            header += f" [{workload_name}]"
        header += " ==="
        print(header)
        print(f"  packets created   : {self.total_packets_created}")
        print(f"  packets delivered : {self.total_packets_delivered}  "
              f"({self.delivery_rate()*100:.1f}%)")
        print(f"  packets dropped   : {self.total_packets_dropped}  "
              f"({self.drop_rate()*100:.1f}%)")
        print(f"  flows completed   : {self.completed_flows}/{self.total_flows}")
        print(f"  avg FCT           : {self.average_flow_completion_time:.2f} ticks")
        print(f"  max FCT           : {self.max_flow_completion_time} ticks")
        print(f"  p99 FCT           : {self.p99_flow_completion_time} ticks")
        print(f"  avg queue length  : {self.average_queue_length:.3f} pkts")
        print(f"  max queue length  : {self.max_queue_length} pkts")
        if self.spine_utilization:
            print("  spine utilization :")
            for spine_id, util in sorted(self.spine_utilization.items()):
                bar = "#" * int(util * 20)
                print(f"    {spine_id:12s} {util:.3f}  |{bar:<20}|")
        # Top 5 most-loaded links
        top = sorted(self.link_stats, key=lambda s: s.utilization, reverse=True)[:5]
        if top:
            print("  top-5 busy links  :")
            for ls in top:
                bar = "#" * int(ls.utilization * 20)
                print(f"    {ls.link_id:30s}  util={ls.utilization:.3f}  "
                      f"drop={ls.dropped}  avgQ={ls.avg_queue_length:.2f}")


def _percentile(values: list[int], pct: float) -> int:
    if not values:
        return 0
    sorted_vals = sorted(values)
    idx = max(0, int(len(sorted_vals) * pct / 100) - 1)
    return sorted_vals[idx]


def build_report(
    flows: list[Flow],
    links: list[Link],
    total_packets_created: int,
    spine_switch_ids: list[str] | None = None,
) -> SimulationReport:
    total_packets_delivered = sum(flow.packets_delivered for flow in flows)
    total_packets_dropped = sum(link.dropped_packets for link in links)

    completion_times: list[int] = []
    for flow in flows:
        if flow.completion_tick is not None:
            completion_times.append(flow.completion_tick - flow.start_tick)

    avg_completion = 0.0
    max_completion = 0
    p99_completion = 0
    if completion_times:
        avg_completion = sum(completion_times) / len(completion_times)
        max_completion = max(completion_times)
        p99_completion = _percentile(completion_times, 99)

    # Per-link stats
    link_stats: list[LinkStats] = []
    for link in links:
        if link.queue_length_samples == 0:
            util = 0.0
            max_q = 0
        else:
            # utilization = fraction of ticks the queue was non-empty
            util = link.transmitted_packets / max(1, link.queue_length_samples)
            util = min(1.0, util / link.capacity_packets_per_tick)
            max_q = link._max_queue_length  # type: ignore[attr-defined]
        link_stats.append(LinkStats(
            link_id=f"{link.src}->{link.dst}",
            transmitted=link.transmitted_packets,
            dropped=link.dropped_packets,
            utilization=util,
            avg_queue_length=link.average_queue_length,
            max_queue_length=max_q,
        ))

    # Aggregate queue stats
    all_avg_qs = [ls.avg_queue_length for ls in link_stats]
    all_max_qs = [ls.max_queue_length for ls in link_stats]
    overall_avg_q = sum(all_avg_qs) / max(1, len(all_avg_qs))
    overall_max_q = max(all_max_qs) if all_max_qs else 0

    # Spine utilization
    spine_util: dict[str, float] = {}
    if spine_switch_ids:
        for ls in link_stats:
            src = ls.link_id.split("->")[0]
            if src in spine_switch_ids:
                spine_util[src] = spine_util.get(src, 0.0) + ls.transmitted
        # Normalise to fraction of total spine-outbound packets
        total_spine_tx = sum(spine_util.values()) or 1
        spine_util = {k: v / total_spine_tx for k, v in spine_util.items()}

    return SimulationReport(
        total_packets_created=total_packets_created,
        total_packets_delivered=total_packets_delivered,
        total_packets_dropped=total_packets_dropped,
        completed_flows=sum(1 for flow in flows if flow.is_complete),
        total_flows=len(flows),
        average_flow_completion_time=avg_completion,
        max_flow_completion_time=max_completion,
        p99_flow_completion_time=p99_completion,
        average_queue_length=overall_avg_q,
        max_queue_length=overall_max_q,
        link_stats=link_stats,
        spine_utilization=spine_util,
    )

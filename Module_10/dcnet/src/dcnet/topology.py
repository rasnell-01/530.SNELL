from __future__ import annotations

from dataclasses import dataclass, field

from .config import SimConfig
from .model import Link, Node, NodeKind


@dataclass(slots=True)
class Topology:
    nodes: dict[str, Node] = field(default_factory=dict)
    links: dict[tuple[str, str], Link] = field(default_factory=dict)
    server_to_leaf: dict[str, str] = field(default_factory=dict)
    leaf_to_servers: dict[str, list[str]] = field(default_factory=dict)
    leaf_switches: list[str] = field(default_factory=list)
    spine_switches: list[str] = field(default_factory=list)

    def add_node(self, node: Node) -> None:
        self.nodes[node.node_id] = node

    def add_link(self, src: str, dst: str, cfg: SimConfig) -> None:
        self.links[(src, dst)] = Link(
            src=src,
            dst=dst,
            capacity_packets_per_tick=cfg.link_bandwidth_packets_per_tick,
            queue_capacity_packets=cfg.queue_capacity_packets,
        )

    def neighbors(self, node_id: str) -> list[str]:
        return [dst for (src, dst) in self.links if src == node_id]



def build_leaf_spine_topology(cfg: SimConfig) -> Topology:
    topo = Topology()

    for spine_index in range(cfg.num_spine_switches):
        spine_id = f"spine{spine_index}"
        topo.add_node(Node(spine_id, NodeKind.SPINE))
        topo.spine_switches.append(spine_id)

    for leaf_index in range(cfg.num_leaf_switches):
        leaf_id = f"leaf{leaf_index}"
        topo.add_node(Node(leaf_id, NodeKind.LEAF))
        topo.leaf_switches.append(leaf_id)
        topo.leaf_to_servers[leaf_id] = []

        for server_index in range(cfg.servers_per_leaf):
            server_id = f"server{leaf_index}_{server_index}"
            topo.add_node(Node(server_id, NodeKind.SERVER))
            topo.server_to_leaf[server_id] = leaf_id
            topo.leaf_to_servers[leaf_id].append(server_id)

            topo.add_link(server_id, leaf_id, cfg)
            topo.add_link(leaf_id, server_id, cfg)

        for spine_id in topo.spine_switches:
            topo.add_link(leaf_id, spine_id, cfg)
            topo.add_link(spine_id, leaf_id, cfg)

    return topo

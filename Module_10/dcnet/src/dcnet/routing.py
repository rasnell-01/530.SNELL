from __future__ import annotations

import hashlib

from .topology import Topology


class RoutePlanner:
    """Computes next hops for packets in a leaf-spine topology.

    ECMP spine selection uses a hash over (src, dst) so that all flows
    between the same pair always use the same spine.  This is intentionally
    realistic: real ECMP hardware hashes on the 5-tuple, which means two
    heavy flows sharing a src/dst will collide on the same spine even when
    other spines sit idle.

    A secondary ``flow_id`` term is available via use_flow_id_in_hash=True
    (useful for the ECMP-balance experiment where per-flow variation is desired).
    """

    def __init__(self, topology: Topology, use_flow_id_in_hash: bool = False) -> None:
        self.topology = topology
        self.use_flow_id_in_hash = use_flow_id_in_hash
        self.flow_to_spine: dict[int, str] = {}

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _choose_spine_for_flow(self, flow_id: int, src: str, dst: str) -> str:
        """Stable ECMP spine choice, consistent for the lifetime of the flow."""
        if flow_id not in self.flow_to_spine:
            spine_list = self.topology.spine_switches
            if self.use_flow_id_in_hash:
                key = f"{src}:{dst}:{flow_id}".encode()
            else:
                # Hash only on (src, dst): all flows sharing a src/dst pair land
                # on the same spine, which exposes ECMP collisions.
                key = f"{src}:{dst}".encode()
            digest = int(hashlib.md5(key).hexdigest(), 16)
            self.flow_to_spine[flow_id] = spine_list[digest % len(spine_list)]
        return self.flow_to_spine[flow_id]

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def next_hop(self, current_node: str, src: str, dst: str, flow_id: int) -> str | None:
        """Return the next node on the path, or None if already at destination."""
        src_leaf = self.topology.server_to_leaf[src]
        dst_leaf = self.topology.server_to_leaf[dst]

        if current_node == src:
            return src_leaf

        if current_node == src_leaf:
            if src_leaf == dst_leaf:
                return dst        # intra-rack: skip spine entirely
            return self._choose_spine_for_flow(flow_id, src, dst)

        if current_node in self.topology.spine_switches:
            return dst_leaf

        if current_node == dst_leaf:
            return dst

        if current_node == dst:
            return None           # already delivered

        raise ValueError(f"No routing rule for packet at node {current_node!r} "
                         f"(src={src!r}, dst={dst!r})")

    def spine_for_flow(self, flow_id: int, src: str, dst: str) -> str | None:
        """Return the spine used by a cross-rack flow, or None for intra-rack."""
        src_leaf = self.topology.server_to_leaf[src]
        dst_leaf = self.topology.server_to_leaf[dst]
        if src_leaf == dst_leaf:
            return None
        return self._choose_spine_for_flow(flow_id, src, dst)

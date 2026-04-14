from dcnet.config import SimConfig
from dcnet.topology import build_leaf_spine_topology



def test_topology_sizes() -> None:
    cfg = SimConfig(num_leaf_switches=2, servers_per_leaf=3, num_spine_switches=2)
    topo = build_leaf_spine_topology(cfg)

    assert len(topo.leaf_switches) == 2
    assert len(topo.spine_switches) == 2
    assert len(topo.server_to_leaf) == 6

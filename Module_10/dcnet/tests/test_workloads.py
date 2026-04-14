from dcnet.config import SimConfig
from dcnet.topology import build_leaf_spine_topology
from dcnet.workloads import make_incast_workload, make_uniform_random_workload



def test_uniform_workload_generates_flows() -> None:
    cfg = SimConfig()
    topo = build_leaf_spine_topology(cfg)
    flows = make_uniform_random_workload(topo, cfg, num_flows=5)
    assert len(flows) == 5
    assert all(flow.src != flow.dst for flow in flows)



def test_incast_workload_same_receiver() -> None:
    cfg = SimConfig()
    topo = build_leaf_spine_topology(cfg)
    flows = make_incast_workload(topo, cfg, num_senders=4)
    receivers = {flow.dst for flow in flows}
    assert len(receivers) == 1

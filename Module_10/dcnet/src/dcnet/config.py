from dataclasses import dataclass


@dataclass(slots=True)
class SimConfig:
    # Topology
    num_leaf_switches: int = 8
    servers_per_leaf: int = 8
    num_spine_switches: int = 4

    # Link / queue parameters
    link_bandwidth_packets_per_tick: int = 4
    queue_capacity_packets: int = 32

    # Traffic generation
    default_flow_size_packets: int = 20
    inject_rate_packets_per_tick: int = 1   # max new packets injected per flow per tick

    # Simulation duration
    ticks: int = 200

    # Reproducibility
    random_seed: int = 7

    # Routing: when True, hash includes flow_id for per-flow ECMP variation
    ecmp_use_flow_id: bool = False

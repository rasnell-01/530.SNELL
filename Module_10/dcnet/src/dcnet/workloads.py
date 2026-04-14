from __future__ import annotations

import random
from typing import Iterable

from .config import SimConfig
from .model import Flow
from .topology import Topology


def all_servers(topology: Topology) -> list[str]:
    return list(topology.server_to_leaf.keys())


def make_uniform_random_workload(
    topology: Topology,
    cfg: SimConfig,
    num_flows: int = 20,
) -> list[Flow]:
    """Each flow picks a random (src, dst) pair. Traffic is spread uniformly."""
    rng = random.Random(cfg.random_seed)
    servers = all_servers(topology)
    flows: list[Flow] = []

    for flow_id in range(num_flows):
        src = rng.choice(servers)
        dst = rng.choice(servers)
        while dst == src:
            dst = rng.choice(servers)
        flows.append(
            Flow(
                flow_id=flow_id,
                src=src,
                dst=dst,
                size_packets=cfg.default_flow_size_packets,
                start_tick=rng.randint(0, 5),
            )
        )
    return flows


def make_incast_workload(
    topology: Topology,
    cfg: SimConfig,
    num_senders: int = 12,
    start_tick: int = 0,
) -> list[Flow]:
    """All senders transmit simultaneously to a single receiver (incast scenario).

    All flows begin at ``start_tick`` to maximise the burst at the receiver's
    downlink — this is exactly the condition that triggers incast collapse.
    """
    rng = random.Random(cfg.random_seed)
    servers = all_servers(topology)
    receiver = rng.choice(servers)
    senders = [s for s in servers if s != receiver]
    rng.shuffle(senders)
    senders = senders[:num_senders]

    flows: list[Flow] = []
    for flow_id, sender in enumerate(senders):
        flows.append(
            Flow(
                flow_id=flow_id,
                src=sender,
                dst=receiver,
                size_packets=cfg.default_flow_size_packets,
                start_tick=start_tick,
            )
        )
    return flows


def make_hotspot_workload(
    topology: Topology,
    cfg: SimConfig,
    num_flows: int = 24,
    hotspot_fraction: float = 0.7,
    num_hot_receivers: int = 2,
) -> list[Flow]:
    """A fraction ``hotspot_fraction`` of flows target a small set of hot receivers.

    Increasing ``hotspot_fraction`` toward 1.0 concentrates more traffic on
    the hot receivers and their uplinks, demonstrating queue buildup.
    """
    rng = random.Random(cfg.random_seed)
    servers = all_servers(topology)
    hot_receivers = rng.sample(servers, k=min(num_hot_receivers, len(servers)))

    flows: list[Flow] = []
    for flow_id in range(num_flows):
        src = rng.choice(servers)
        if rng.random() < hotspot_fraction:
            dst = rng.choice(hot_receivers)
        else:
            dst = rng.choice(servers)
        while dst == src:
            dst = rng.choice(servers)
        flows.append(
            Flow(
                flow_id=flow_id,
                src=src,
                dst=dst,
                size_packets=cfg.default_flow_size_packets,
                start_tick=rng.randint(0, 5),
            )
        )
    return flows


def workload_by_name(
    name: str,
    topology: Topology,
    cfg: SimConfig,
    **kwargs,
) -> list[Flow]:
    normalized = name.strip().lower()
    if normalized == "uniform":
        return make_uniform_random_workload(topology, cfg, **kwargs)
    if normalized == "incast":
        return make_incast_workload(topology, cfg, **kwargs)
    if normalized == "hotspot":
        return make_hotspot_workload(topology, cfg, **kwargs)
    raise ValueError(f"Unknown workload name: {name!r}")

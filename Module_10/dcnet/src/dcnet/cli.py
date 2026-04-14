from __future__ import annotations

import argparse

from .config import SimConfig
from .simulator import Simulator
from .topology import build_leaf_spine_topology
from .workloads import workload_by_name


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Leaf-spine data center network simulator")
    parser.add_argument("--workload", choices=["uniform", "incast", "hotspot"],
                        default="uniform")
    parser.add_argument("--ticks", type=int, default=200)
    parser.add_argument("--leafs", type=int, default=8)
    parser.add_argument("--servers-per-leaf", type=int, default=8)
    parser.add_argument("--spines", type=int, default=4)
    parser.add_argument("--bandwidth", type=int, default=4,
                        help="Link capacity in packets/tick")
    parser.add_argument("--queue-capacity", type=int, default=32,
                        help="Queue buffer size in packets")
    parser.add_argument("--inject-rate", type=int, default=1,
                        help="Max packets injected per flow per tick")
    parser.add_argument("--num-flows", type=int, default=20,
                        help="Number of flows (uniform/hotspot workloads)")
    parser.add_argument("--num-senders", type=int, default=12,
                        help="Number of incast senders")
    parser.add_argument("--hotspot-fraction", type=float, default=0.7,
                        help="Fraction of flows directed to hot receivers")
    parser.add_argument("--ecmp-flow-id", action="store_true",
                        help="Include flow_id in ECMP hash for per-flow variation")
    parser.add_argument("--seed", type=int, default=7)
    parser.add_argument("--verbose", "-v", action="store_true",
                        help="Print per-link utilization table")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    cfg = SimConfig(
        num_leaf_switches=args.leafs,
        servers_per_leaf=args.servers_per_leaf,
        num_spine_switches=args.spines,
        link_bandwidth_packets_per_tick=args.bandwidth,
        queue_capacity_packets=args.queue_capacity,
        inject_rate_packets_per_tick=args.inject_rate,
        ticks=args.ticks,
        random_seed=args.seed,
        ecmp_use_flow_id=args.ecmp_flow_id,
    )

    topology = build_leaf_spine_topology(cfg)

    workload_kwargs: dict = {}
    if args.workload == "incast":
        workload_kwargs["num_senders"] = args.num_senders
    elif args.workload == "uniform":
        workload_kwargs["num_flows"] = args.num_flows
    elif args.workload == "hotspot":
        workload_kwargs["num_flows"] = args.num_flows
        workload_kwargs["hotspot_fraction"] = args.hotspot_fraction

    flows = workload_by_name(args.workload, topology, cfg, **workload_kwargs)
    simulator = Simulator(topology=topology, cfg=cfg, flows=flows)
    report = simulator.run()
    report.print_summary(args.workload)

    if args.verbose:
        print("\n--- Per-Link Utilization (top 20) ---")
        top = sorted(report.link_stats, key=lambda s: s.utilization, reverse=True)[:20]
        print(f"{'Link':<35} {'Util':>6}  {'Tx':>6}  {'Drop':>6}  {'AvgQ':>6}  {'MaxQ':>5}")
        print("-" * 75)
        for ls in top:
            print(f"{ls.link_id:<35} {ls.utilization:>6.3f}  {ls.transmitted:>6}  "
                  f"{ls.dropped:>6}  {ls.avg_queue_length:>6.2f}  {ls.max_queue_length:>5}")


if __name__ == "__main__":
    main()

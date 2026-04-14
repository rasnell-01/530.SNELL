"""
experiments.py — Runs all six Part-6 experiments and prints tabular results.

Run from the project root:
    python experiments.py

Each experiment sweeps one parameter and prints a results table.
Optional matplotlib graphs are produced if matplotlib is installed.
"""

from __future__ import annotations

import sys
import statistics

# ── make the src layout importable ──────────────────────────────────────────
sys.path.insert(0, "src")

from dcnet.config import SimConfig
from dcnet.simulator import Simulator
from dcnet.topology import build_leaf_spine_topology
from dcnet.workloads import (
    make_incast_workload,
    make_uniform_random_workload,
    make_hotspot_workload,
)

try:
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    HAS_MPL = True
except ImportError:
    HAS_MPL = False


# ── helpers ──────────────────────────────────────────────────────────────────

def run_once(cfg: SimConfig, flows_fn, **kw):
    topo = build_leaf_spine_topology(cfg)
    flows = flows_fn(topo, cfg, **kw)
    sim = Simulator(topology=topo, cfg=cfg, flows=flows)
    return sim.run(), topo


def header(title: str) -> None:
    print()
    print("=" * 70)
    print(f"  {title}")
    print("=" * 70)


def row(*cols, widths=None):
    if widths is None:
        widths = [14] * len(cols)
    parts = [str(c).rjust(w) for c, w in zip(cols, widths)]
    print("  ".join(parts))


# ─────────────────────────────────────────────────────────────────────────────
# Experiment 1: Incast — FCT vs number of simultaneous senders
# ─────────────────────────────────────────────────────────────────────────────

def exp1_incast_vs_senders():
    header("Experiment 1: Incast — FCT vs number of simultaneous senders")
    row("Senders", "Avg FCT", "Max FCT", "P99 FCT", "Dropped", "Drop%",
        widths=[10, 10, 10, 10, 10, 8])
    row("-------", "-------", "-------", "-------", "-------", "-----",
        widths=[10, 10, 10, 10, 10, 8])

    sender_counts = [1, 2, 4, 8, 12, 16, 20, 24, 32]
    results = []
    cfg = SimConfig(ticks=300, queue_capacity_packets=32,
                    link_bandwidth_packets_per_tick=4,
                    default_flow_size_packets=20)

    for n in sender_counts:
        report, _ = run_once(cfg, make_incast_workload, num_senders=n)
        results.append((n, report.average_flow_completion_time,
                        report.max_flow_completion_time,
                        report.p99_flow_completion_time,
                        report.total_packets_dropped,
                        report.drop_rate() * 100))
        row(n,
            f"{report.average_flow_completion_time:.1f}",
            report.max_flow_completion_time,
            report.p99_flow_completion_time,
            report.total_packets_dropped,
            f"{report.drop_rate()*100:.1f}%",
            widths=[10, 10, 10, 10, 10, 8])

    if HAS_MPL:
        fig, axes = plt.subplots(1, 2, figsize=(10, 4))
        xs = [r[0] for r in results]
        axes[0].plot(xs, [r[1] for r in results], "o-", label="Avg FCT")
        axes[0].plot(xs, [r[2] for r in results], "s--", label="Max FCT")
        axes[0].set_xlabel("Number of incast senders")
        axes[0].set_ylabel("Flow Completion Time (ticks)")
        axes[0].set_title("Exp 1: Incast FCT vs Senders")
        axes[0].legend()
        axes[0].grid(True, alpha=0.3)

        axes[1].bar(xs, [r[5] for r in results])
        axes[1].set_xlabel("Number of incast senders")
        axes[1].set_ylabel("Drop rate (%)")
        axes[1].set_title("Exp 1: Drop Rate vs Senders")
        axes[1].grid(True, alpha=0.3)

        plt.tight_layout()
        plt.savefig("exp1_incast.png", dpi=120)
        print("  [graph saved: exp1_incast.png]")
    return results


# ─────────────────────────────────────────────────────────────────────────────
# Experiment 2: ECMP balance under uniform traffic
# ─────────────────────────────────────────────────────────────────────────────

def exp2_ecmp_balance():
    header("Experiment 2: ECMP balance under uniform traffic")
    print("  Comparing spine utilization: hash(src,dst) vs hash(src,dst,flow_id)")
    print()

    cfg_no_fid  = SimConfig(ticks=300, ecmp_use_flow_id=False,
                            default_flow_size_packets=20)
    cfg_with_fid = SimConfig(ticks=300, ecmp_use_flow_id=True,
                             default_flow_size_packets=20)

    for label, cfg in [("hash(src,dst) — collision-prone", cfg_no_fid),
                       ("hash(src,dst,flow_id) — per-flow variation", cfg_with_fid)]:
        report, topo = run_once(cfg, make_uniform_random_workload, num_flows=40)
        spines = topo.spine_switches
        print(f"  {label}")
        spine_tx: dict[str, int] = {s: 0 for s in spines}
        for ls in report.link_stats:
            src = ls.link_id.split("->")[0]
            if src in spine_tx:
                spine_tx[src] += ls.transmitted
        total_tx = sum(spine_tx.values()) or 1
        vals = [spine_tx[s] / total_tx for s in spines]
        for s in spines:
            pct = spine_tx[s] / total_tx * 100
            bar = "#" * int(pct / 2)
            print(f"    {s:10s}  {pct:5.1f}%  |{bar:<50}|")
        if len(vals) > 1:
            cv = statistics.stdev(vals) / (statistics.mean(vals) or 1)
            print(f"    Coefficient of variation (lower=more balanced): {cv:.4f}")
        print()

    if HAS_MPL:
        fig, axes = plt.subplots(1, 2, figsize=(10, 4))
        for ax, (label, cfg) in zip(axes, [
            ("hash(src,dst)", cfg_no_fid),
            ("hash(src,dst,flow_id)", cfg_with_fid),
        ]):
            report, topo = run_once(cfg, make_uniform_random_workload, num_flows=40)
            spines = topo.spine_switches
            spine_tx = {s: 0 for s in spines}
            for ls in report.link_stats:
                src = ls.link_id.split("->")[0]
                if src in spine_tx:
                    spine_tx[src] += ls.transmitted
            total = sum(spine_tx.values()) or 1
            ax.bar(list(spine_tx.keys()),
                   [spine_tx[s] / total * 100 for s in spine_tx])
            ax.set_title(f"Exp 2: {label}")
            ax.set_ylabel("% of spine-outbound packets")
            ax.set_ylim(0, 60)
            ax.grid(True, alpha=0.3)
        plt.tight_layout()
        plt.savefig("exp2_ecmp_balance.png", dpi=120)
        print("  [graph saved: exp2_ecmp_balance.png]")


# ─────────────────────────────────────────────────────────────────────────────
# Experiment 3: ECMP can still overload links — uniform traffic, many flows
# ─────────────────────────────────────────────────────────────────────────────

def exp3_ecmp_overload():
    header("Experiment 3: ECMP overload — utilization spread across spine links")

    cfg = SimConfig(ticks=400, ecmp_use_flow_id=False,
                    default_flow_size_packets=30, link_bandwidth_packets_per_tick=4)
    report, topo = run_once(cfg, make_uniform_random_workload, num_flows=60)

    # Show utilization of every spine-adjacent link
    spines = set(topo.spine_switches)
    spine_links = [ls for ls in report.link_stats
                   if ls.link_id.split("->")[0] in spines
                   or ls.link_id.split("->")[1] in spines]

    row("Link", "Util", "Tx", "Drop", "AvgQ",
        widths=[35, 7, 7, 7, 7])
    row("----", "----", "--", "----", "----",
        widths=[35, 7, 7, 7, 7])
    for ls in sorted(spine_links, key=lambda x: x.utilization, reverse=True)[:20]:
        row(ls.link_id, f"{ls.utilization:.3f}", ls.transmitted,
            ls.dropped, f"{ls.avg_queue_length:.2f}",
            widths=[35, 7, 7, 7, 7])

    utils = [ls.utilization for ls in spine_links]
    if utils:
        print(f"\n  Max spine-link util : {max(utils):.3f}")
        print(f"  Min spine-link util : {min(utils):.3f}")
        print(f"  Ratio max/min       : {max(utils)/max(min(utils), 1e-6):.2f}x")

    if HAS_MPL:
        spine_out = [ls for ls in spine_links if ls.link_id.split("->")[0] in spines]
        labels = [ls.link_id.replace("->", "→\n") for ls in spine_out]
        utils_out = [ls.utilization for ls in spine_out]
        fig, ax = plt.subplots(figsize=(10, 4))
        ax.bar(range(len(labels)), utils_out)
        ax.set_xticks(range(len(labels)))
        ax.set_xticklabels(labels, fontsize=7, rotation=45, ha="right")
        ax.axhline(sum(utils_out) / max(len(utils_out), 1), color="red",
                   linestyle="--", label="mean")
        ax.set_ylabel("Utilization")
        ax.set_title("Exp 3: Spine outbound link utilization (uniform, 60 flows)")
        ax.legend()
        ax.grid(True, alpha=0.3)
        plt.tight_layout()
        plt.savefig("exp3_ecmp_overload.png", dpi=120)
        print("  [graph saved: exp3_ecmp_overload.png]")


# ─────────────────────────────────────────────────────────────────────────────
# Experiment 4: Effect of queue size on FCT and drop rate
# ─────────────────────────────────────────────────────────────────────────────

def exp4_queue_size():
    header("Experiment 4: Effect of queue buffer size")
    row("QueueSz", "Avg FCT", "Max FCT", "AvgQ", "MaxQ", "Drop%",
        widths=[9, 10, 10, 8, 8, 8])
    row("-------", "-------", "-------", "----", "----", "-----",
        widths=[9, 10, 10, 8, 8, 8])

    queue_sizes = [4, 8, 16, 32, 64, 128, 256]
    results = []

    for qs in queue_sizes:
        cfg = SimConfig(ticks=400, queue_capacity_packets=qs,
                        link_bandwidth_packets_per_tick=4,
                        default_flow_size_packets=20)
        report, _ = run_once(cfg, make_incast_workload, num_senders=16)
        results.append((qs, report.average_flow_completion_time,
                        report.max_flow_completion_time,
                        report.average_queue_length,
                        report.max_queue_length,
                        report.drop_rate() * 100))
        row(qs,
            f"{report.average_flow_completion_time:.1f}",
            report.max_flow_completion_time,
            f"{report.average_queue_length:.3f}",
            report.max_queue_length,
            f"{report.drop_rate()*100:.1f}%",
            widths=[9, 10, 10, 8, 8, 8])

    if HAS_MPL:
        fig, axes = plt.subplots(1, 3, figsize=(13, 4))
        xs = [r[0] for r in results]
        axes[0].plot(xs, [r[1] for r in results], "o-")
        axes[0].set_xscale("log", base=2)
        axes[0].set_xlabel("Queue capacity (packets)")
        axes[0].set_ylabel("Avg FCT (ticks)")
        axes[0].set_title("Exp 4: Avg FCT vs Queue Size")
        axes[0].grid(True, alpha=0.3)

        axes[1].plot(xs, [r[5] for r in results], "s-", color="red")
        axes[1].set_xscale("log", base=2)
        axes[1].set_xlabel("Queue capacity (packets)")
        axes[1].set_ylabel("Drop rate (%)")
        axes[1].set_title("Exp 4: Drop Rate vs Queue Size")
        axes[1].grid(True, alpha=0.3)

        axes[2].plot(xs, [r[3] for r in results], "^-", color="green")
        axes[2].set_xscale("log", base=2)
        axes[2].set_xlabel("Queue capacity (packets)")
        axes[2].set_ylabel("Avg queue length (pkts)")
        axes[2].set_title("Exp 4: Avg Queue vs Queue Size")
        axes[2].grid(True, alpha=0.3)

        plt.tight_layout()
        plt.savefig("exp4_queue_size.png", dpi=120)
        print("  [graph saved: exp4_queue_size.png]")
    return results


# ─────────────────────────────────────────────────────────────────────────────
# Experiment 5: Hotspot concentration
# ─────────────────────────────────────────────────────────────────────────────

def exp5_hotspot_concentration():
    header("Experiment 5: Hotspot traffic concentration")
    row("HotFrac", "Avg FCT", "Max FCT", "Dropped", "Drop%", "AvgQ",
        widths=[9, 10, 10, 10, 8, 8])
    row("-------", "-------", "-------", "-------", "-----", "----",
        widths=[9, 10, 10, 10, 8, 8])

    fractions = [0.0, 0.1, 0.2, 0.3, 0.5, 0.7, 0.85, 1.0]
    results = []

    cfg = SimConfig(ticks=400, queue_capacity_packets=32,
                    link_bandwidth_packets_per_tick=4,
                    default_flow_size_packets=20)

    for frac in fractions:
        report, _ = run_once(cfg, make_hotspot_workload,
                             num_flows=32, hotspot_fraction=frac, num_hot_receivers=2)
        results.append((frac, report.average_flow_completion_time,
                        report.max_flow_completion_time,
                        report.total_packets_dropped,
                        report.drop_rate() * 100,
                        report.average_queue_length))
        row(f"{frac:.2f}",
            f"{report.average_flow_completion_time:.1f}",
            report.max_flow_completion_time,
            report.total_packets_dropped,
            f"{report.drop_rate()*100:.1f}%",
            f"{report.average_queue_length:.3f}",
            widths=[9, 10, 10, 10, 8, 8])

    if HAS_MPL:
        fig, axes = plt.subplots(1, 2, figsize=(10, 4))
        xs = [r[0] for r in results]
        axes[0].plot(xs, [r[1] for r in results], "o-", label="Avg FCT")
        axes[0].plot(xs, [r[2] for r in results], "s--", label="Max FCT")
        axes[0].set_xlabel("Hotspot fraction")
        axes[0].set_ylabel("Flow Completion Time (ticks)")
        axes[0].set_title("Exp 5: FCT vs Hotspot Fraction")
        axes[0].legend()
        axes[0].grid(True, alpha=0.3)

        axes[1].plot(xs, [r[4] for r in results], "^-", color="red")
        axes[1].set_xlabel("Hotspot fraction")
        axes[1].set_ylabel("Drop rate (%)")
        axes[1].set_title("Exp 5: Drop Rate vs Hotspot Fraction")
        axes[1].grid(True, alpha=0.3)

        plt.tight_layout()
        plt.savefig("exp5_hotspot.png", dpi=120)
        print("  [graph saved: exp5_hotspot.png]")
    return results


# ─────────────────────────────────────────────────────────────────────────────
# Main
# ─────────────────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    print("Leaf-Spine DC Simulator — Experiment Suite")
    print(f"matplotlib: {'available' if HAS_MPL else 'NOT found (install with: pip install matplotlib)'}")

    exp1_incast_vs_senders()
    exp2_ecmp_balance()
    exp3_ecmp_overload()
    exp4_queue_size()
    exp5_hotspot_concentration()

    print("\nAll experiments complete.")

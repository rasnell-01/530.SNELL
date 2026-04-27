#!/usr/bin/env python3
"""
TCP/IP In Space — Protocol Simulator
Demonstrates why TCP/IP performs poorly in space environments,
and how DTN (Delay/Disruption Tolerant Networking) solves these problems.

Usage:
    python simulator.py --experiment 1
    python simulator.py --experiment 2
    python simulator.py --experiment 3
    python simulator.py --all
    python simulator.py --all --plot
"""

import random
import argparse
from dataclasses import dataclass
from typing import List, Tuple, Dict, Set

# ─────────────────────────────────────────────────────────────────────────────
# Constants
# ─────────────────────────────────────────────────────────────────────────────

DATA_SIZE    = 10_000   # total payload bytes
SEGMENT_SIZE = 1_000    # bytes per TCP segment  → 10 segments total
BUNDLE_SIZE  = 2_500    # bytes per DTN bundle   →  4 bundles total
RTO          = 150      # TCP retransmission timeout (sim-seconds)
WINDOW_SIZE  = 4        # TCP max in-flight segments (sliding window)
MAX_TIME     = 4_000    # simulation time horizon (sim-seconds)


# ─────────────────────────────────────────────────────────────────────────────
# Link
# ─────────────────────────────────────────────────────────────────────────────

class Link:
    """
    Uni-directional network link.

    Attributes:
        name      : human-readable label
        delay     : one-way propagation delay (sim-seconds)
        windows   : list of (start, end) availability windows (inclusive)
        loss_prob : per-packet independent loss probability [0.0, 1.0)
    """

    def __init__(self, name: str, delay: int,
                 windows: List[Tuple[int, int]], loss_prob: float = 0.0):
        self.name      = name
        self.delay     = delay
        self.windows   = windows
        self.loss_prob = loss_prob

    def is_available(self, t: int) -> bool:
        return any(s <= t <= e for s, e in self.windows)

    def will_deliver(self) -> bool:
        """Simulate independent packet loss. True means packet survives."""
        return random.random() >= self.loss_prob


# ─────────────────────────────────────────────────────────────────────────────
# Metrics
# ─────────────────────────────────────────────────────────────────────────────

@dataclass
class Metrics:
    mode:               str
    experiment_name:    str
    delivery_time:      int   = MAX_TIME
    retransmissions:    int   = 0
    units_delivered:    int   = 0
    units_total:        int   = 0
    wait_cycles:        int   = 0
    max_stored_bundles: int   = 0      # DTN only

    @property
    def delivery_pct(self) -> float:
        return 100.0 * self.units_delivered / max(1, self.units_total)

    def __str__(self) -> str:
        dt = f"{self.delivery_time} s" if self.units_delivered == self.units_total else f"{self.delivery_time} s (incomplete)"
        lines = [
            f"  Protocol          : {self.mode}",
            f"  Experiment        : {self.experiment_name}",
            f"  Delivery time     : {dt}",
            f"  Data delivered    : {self.units_delivered}/{self.units_total}  ({self.delivery_pct:.1f}%)",
            f"  Retransmissions   : {self.retransmissions}",
            f"  Wait cycles       : {self.wait_cycles} s",
        ]
        if self.mode == "DTN":
            lines.append(f"  Peak stored bundles: {self.max_stored_bundles}")
        return "\n".join(lines)


# ─────────────────────────────────────────────────────────────────────────────
# TCP Simulator
# ─────────────────────────────────────────────────────────────────────────────

def run_tcp(link_er: Link, link_ro: Link, exp_name: str) -> Metrics:
    """
    Simplified TCP-style simulation.

    Key behaviors modeled:
    - Earth maintains a sliding window of WINDOW_SIZE segments.
    - Segments travel Earth → Orbiter (link_er) → Rover (link_ro).
    - Orbiter is a STATELESS IP router — if link_ro is down when a segment
      arrives, the segment is DROPPED (no store-and-forward).
    - ACKs return via the reverse path (delay = link_ro.delay + link_er.delay).
    - If no ACK arrives within RTO seconds, the sender retransmits.
    - End-to-end connectivity is REQUIRED: both links must be up for progress.
    """
    num_seg = DATA_SIZE // SEGMENT_SIZE
    m = Metrics(mode="TCP", experiment_name=exp_name, units_total=num_seg)

    last_sent:       Dict[int, int]        = {}    # seq → last send time
    acked:           Set[int]              = set()
    acks_in_flight:  List[Tuple[int, int]] = []    # (arrive_t, seq)
    cum_ack:         int                   = 0     # all seqs < cum_ack are acked

    ACK_DELAY = link_ro.delay + link_er.delay      # reverse-path RTT contribution

    for t in range(MAX_TIME):
        er_up = link_er.is_available(t)

        # Count time steps where neither link is usable
        if not er_up and not link_ro.is_available(t):
            m.wait_cycles += 1

        # ── Receive ACKs ────────────────────────────────────────────────────
        remaining = []
        for (at, seq) in acks_in_flight:
            if t >= at:
                if seq not in acked:
                    acked.add(seq)
                    m.units_delivered += 1
                    m.delivery_time = t
                    while cum_ack in acked:     # advance cumulative ACK
                        cum_ack += 1
            else:
                remaining.append((at, seq))
        acks_in_flight = remaining

        if len(acked) == num_seg:
            break

        # ── Send / retransmit within sliding window ──────────────────────────
        if er_up:
            for seq in range(cum_ack, min(cum_ack + WINDOW_SIZE, num_seg)):
                if seq in acked:
                    continue
                # Send on first attempt, or after RTO
                timed_out = (seq not in last_sent) or (t - last_sent[seq] >= RTO)
                if timed_out:
                    if seq in last_sent:
                        m.retransmissions += 1
                    last_sent[seq] = t

                    # Segment arrives at Orbiter at t + link_er.delay.
                    # TCP requires link_ro to be available AT THAT MOMENT.
                    orbiter_arrive = t + link_er.delay
                    if link_ro.is_available(orbiter_arrive):
                        # Independent per-hop loss check
                        if link_er.will_deliver() and link_ro.will_deliver():
                            rover_arrive = orbiter_arrive + link_ro.delay
                            ack_arrive   = rover_arrive + ACK_DELAY
                            acks_in_flight.append((ack_arrive, seq))

    return m


# ─────────────────────────────────────────────────────────────────────────────
# DTN Simulator
# ─────────────────────────────────────────────────────────────────────────────

def run_dtn(link_er: Link, link_ro: Link, exp_name: str) -> Metrics:
    """
    Simplified DTN (Delay/Disruption-Tolerant Networking) simulation.

    Key behaviors modeled:
    - Earth sends all bundles as soon as link_er becomes available.
    - Orbiter persistently stores received bundles (store-and-forward).
    - When link_ro becomes available, Orbiter forwards all stored bundles.
    - No end-to-end connectivity is required — each link hop is independent.
    - No retransmission needed at the application layer.
    """
    num_bun = DATA_SIZE // BUNDLE_SIZE
    m = Metrics(mode="DTN", experiment_name=exp_name, units_total=num_bun)

    earth_sent:     Set[int]              = set()
    orbiter_store:  Set[int]              = set()
    rover_received: Set[int]              = set()
    forwarded:      Set[int]              = set()   # bundles queued for Rover

    er_in_flight: List[Tuple[int, int]]   = []      # (arrive_t, bundle_id)
    ro_in_flight: List[Tuple[int, int]]   = []      # (arrive_t, bundle_id)

    for t in range(MAX_TIME):
        er_up = link_er.is_available(t)
        ro_up = link_ro.is_available(t)

        if not er_up and not ro_up:
            m.wait_cycles += 1

        # ── Earth → Orbiter: transmit all unsent bundles ─────────────────────
        if er_up:
            for bid in range(num_bun):
                if bid not in earth_sent:
                    earth_sent.add(bid)
                    if link_er.will_deliver():
                        er_in_flight.append((t + link_er.delay, bid))

        # ── Deliver arriving bundles to Orbiter store ─────────────────────────
        remaining = []
        for (at, bid) in er_in_flight:
            if t >= at:
                orbiter_store.add(bid)
                m.max_stored_bundles = max(m.max_stored_bundles, len(orbiter_store))
            else:
                remaining.append((at, bid))
        er_in_flight = remaining

        # ── Orbiter → Rover: forward all stored bundles when link is up ───────
        if ro_up:
            for bid in orbiter_store:
                if bid not in forwarded:
                    forwarded.add(bid)
                    if link_ro.will_deliver():
                        ro_in_flight.append((t + link_ro.delay, bid))

        # ── Deliver arriving bundles to Rover ─────────────────────────────────
        remaining = []
        for (at, bid) in ro_in_flight:
            if t >= at:
                if bid not in rover_received:
                    rover_received.add(bid)
                    m.units_delivered += 1
                    m.delivery_time = t
            else:
                remaining.append((at, bid))
        ro_in_flight = remaining

        if len(rover_received) == num_bun:
            break

    return m


# ─────────────────────────────────────────────────────────────────────────────
# Experiment Link Configurations
# ─────────────────────────────────────────────────────────────────────────────

def make_links_exp1(loss: float = 0.0):
    """Experiment 1: Continuous connectivity — both links always available."""
    er = Link("Earth→Orbiter", delay=300, windows=[(0, MAX_TIME)], loss_prob=loss)
    ro = Link("Orbiter→Rover", delay=120, windows=[(0, MAX_TIME)], loss_prob=loss)
    return er, ro


def make_links_exp2(loss: float = 0.0):
    """Experiment 2: Intermittent connectivity — staggered contact windows."""
    er = Link("Earth→Orbiter", delay=300, windows=[(0, 800)],     loss_prob=loss)
    ro = Link("Orbiter→Rover", delay=120, windows=[(1200, 1800)], loss_prob=loss)
    return er, ro


def make_links_exp3(loss: float = 0.08):
    """Experiment 3: Lossy network — 8% loss per hop, continuous links."""
    er = Link("Earth→Orbiter", delay=300, windows=[(0, MAX_TIME)], loss_prob=loss)
    ro = Link("Orbiter→Rover", delay=120, windows=[(0, MAX_TIME)], loss_prob=loss)
    return er, ro


# ─────────────────────────────────────────────────────────────────────────────
# Runner Helpers
# ─────────────────────────────────────────────────────────────────────────────

def run_experiment(number: int, name: str, make_fn) -> Tuple[Metrics, Metrics]:
    print(f"\n{'='*62}")
    print(f"  Experiment {number}: {name}")
    print(f"{'='*62}")

    random.seed(42)
    er, ro = make_fn()
    tcp_m = run_tcp(er, ro, name)

    random.seed(42)
    er, ro = make_fn()
    dtn_m = run_dtn(er, ro, name)

    print("\n[TCP]")
    print(tcp_m)
    print("\n[DTN]")
    print(dtn_m)
    return tcp_m, dtn_m


def print_summary(results: List[Tuple[Metrics, Metrics]]) -> None:
    print(f"\n{'='*76}")
    print("  SUMMARY TABLE")
    print(f"{'='*76}")
    hdr = (f"{'Experiment':<28} {'Proto':<6} {'Delivered':>10} "
           f"{'Del.Time(s)':>12} {'Retrans':>8} {'Wait(s)':>8}")
    print(hdr)
    print("-" * 76)
    for tcp_m, dtn_m in results:
        for m in (tcp_m, dtn_m):
            exp = m.experiment_name[:27]
            print(f"{exp:<28} {m.mode:<6} {m.delivery_pct:>9.1f}%"
                  f" {m.delivery_time:>12} {m.retransmissions:>8} {m.wait_cycles:>8}")
        print()


def try_plot(results: List[Tuple[Metrics, Metrics]]) -> None:
    try:
        import matplotlib
        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
    except ImportError:
        print("\n[plot] matplotlib not installed — skipping plots.")
        return

    labels    = [tcp.experiment_name for tcp, _ in results]
    tcp_times = [tcp.delivery_time   for tcp, _ in results]
    dtn_times = [dtn.delivery_time   for _, dtn in results]
    tcp_pcts  = [tcp.delivery_pct    for tcp, _ in results]
    dtn_pcts  = [dtn.delivery_pct    for _, dtn in results]
    tcp_retx  = [tcp.retransmissions for tcp, _ in results]
    dtn_retx  = [dtn.retransmissions for _, dtn in results]

    x, w = list(range(len(labels))), 0.35
    clr_tcp, clr_dtn = "#e06c75", "#61afef"

    fig, axes = plt.subplots(1, 3, figsize=(15, 5))
    fig.suptitle("TCP vs DTN in Space Environments", fontsize=14, fontweight="bold")

    # Delivery time
    ax = axes[0]
    ax.bar([i - w/2 for i in x], tcp_times, w, label="TCP", color=clr_tcp)
    ax.bar([i + w/2 for i in x], dtn_times, w, label="DTN", color=clr_dtn)
    ax.set_xticks(x); ax.set_xticklabels(labels, rotation=12, ha="right", fontsize=8)
    ax.set_ylabel("Delivery Time (sim-seconds)"); ax.set_title("Total Delivery Time")
    ax.legend()

    # Delivery percentage
    ax = axes[1]
    ax.bar([i - w/2 for i in x], tcp_pcts, w, label="TCP", color=clr_tcp)
    ax.bar([i + w/2 for i in x], dtn_pcts, w, label="DTN", color=clr_dtn)
    ax.set_xticks(x); ax.set_xticklabels(labels, rotation=12, ha="right", fontsize=8)
    ax.set_ylim(0, 115); ax.set_ylabel("Data Delivered (%)")
    ax.set_title("Delivery Percentage"); ax.legend()

    # Retransmissions
    ax = axes[2]
    ax.bar([i - w/2 for i in x], tcp_retx, w, label="TCP", color=clr_tcp)
    ax.bar([i + w/2 for i in x], dtn_retx, w, label="DTN", color=clr_dtn)
    ax.set_xticks(x); ax.set_xticklabels(labels, rotation=12, ha="right", fontsize=8)
    ax.set_ylabel("Retransmissions"); ax.set_title("Retransmissions"); ax.legend()

    plt.tight_layout()
    fname = "results.png"
    plt.savefig(fname, dpi=150)
    print(f"\n[plot] Chart saved → {fname}")


# ─────────────────────────────────────────────────────────────────────────────
# CLI Entry Point
# ─────────────────────────────────────────────────────────────────────────────

EXPERIMENTS = [
    (1, "Continuous Connectivity",   make_links_exp1),
    (2, "Intermittent Connectivity", make_links_exp2),
    (3, "Lossy Network (8%)",        make_links_exp3),
]


def main():
    parser = argparse.ArgumentParser(
        description="TCP vs DTN Space Protocol Simulator",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=(
            "Examples:\n"
            "  python simulator.py --experiment 2\n"
            "  python simulator.py --all\n"
            "  python simulator.py --all --plot\n"
        ),
    )
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--experiment", type=int, choices=[1, 2, 3],
                       metavar="{1,2,3}", help="Run a single experiment")
    group.add_argument("--all", action="store_true", help="Run all three experiments")
    parser.add_argument("--plot", action="store_true",
                        help="Save bar charts to results.png (requires matplotlib)")
    args = parser.parse_args()

    results = []
    if args.all:
        for num, name, fn in EXPERIMENTS:
            results.append(run_experiment(num, name, fn))
        print_summary(results)
    else:
        num, name, fn = EXPERIMENTS[args.experiment - 1]
        results.append(run_experiment(num, name, fn))

    if args.plot:
        try_plot(results)

    print("\nDone.\n")


if __name__ == "__main__":
    main()
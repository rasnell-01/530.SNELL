# Leaf-Spine Data Center Network Simulator

A discrete-event, packet-level simulator for studying incast, ECMP imbalance,
and queueing in a leaf-spine fabric.

## Project Layout

```
src/dcnet/
    config.py       SimConfig dataclass (all tunable parameters)
    model.py        Packet, Flow, Link, NodeKind data structures
    topology.py     build_leaf_spine_topology() — creates nodes + links
    routing.py      RoutePlanner — ECMP hash-based spine selection
    simulator.py    Simulator.run() — tick-driven simulation loop
    workloads.py    Three traffic generators (uniform, incast, hotspot)
    metrics.py      SimulationReport — FCT, queue stats, per-link utilization
    cli.py          Command-line entry point

experiments.py      Runs all Part-6 experiments and saves graphs
tests/              Pytest unit tests
```

## Requirements

```
Python >= 3.10
matplotlib (optional, for graphs)
```

Install dependencies:

```bash
pip install matplotlib pytest
```

## Running the CLI

```bash
# From the project root
python -m src.dcnet.cli --workload uniform
python -m src.dcnet.cli --workload incast  --num-senders 16 --verbose
python -m src.dcnet.cli --workload hotspot --hotspot-fraction 0.8 --verbose
```

### All CLI flags

| Flag | Default | Description |
|---|---|---|
| `--workload` | `uniform` | `uniform`, `incast`, or `hotspot` |
| `--ticks` | `200` | Simulation duration |
| `--leafs` | `8` | Number of leaf switches |
| `--servers-per-leaf` | `8` | Servers per rack |
| `--spines` | `4` | Number of spine switches |
| `--bandwidth` | `4` | Link capacity (packets/tick) |
| `--queue-capacity` | `32` | Queue buffer depth (packets) |
| `--inject-rate` | `1` | Max new packets injected per flow per tick |
| `--num-flows` | `20` | Flows for uniform/hotspot |
| `--num-senders` | `12` | Senders for incast |
| `--hotspot-fraction` | `0.7` | Fraction of flows directed to hot receivers |
| `--ecmp-flow-id` | off | Add flow_id to ECMP hash for per-flow variation |
| `--seed` | `7` | Random seed |
| `--verbose` / `-v` | off | Print full per-link utilization table |

## Running the Experiments

```bash
python experiments.py
```

Prints tabular results for all five Part-6 experiments and saves PNG graphs:
`exp1_incast.png`, `exp2_ecmp_balance.png`, `exp3_ecmp_overload.png`,
`exp4_queue_size.png`, `exp5_hotspot.png`.

## Running Tests

```bash
python -m pytest tests/ -v
```

## Design Notes

**Packet injection** — `_inject_new_packets` creates up to
`inject_rate_packets_per_tick` new packets per active flow per tick and
immediately places them on the source server's uplink queue.

**Two-phase service** — `_service_links` first drains every link queue into a
temporary `arrivals` list (Phase 1), then forwards each arrived packet one hop
forward (Phase 2).  Because draining is complete before any forwarding occurs,
a packet cannot traverse more than one link per tick, which correctly models
propagation delay.

**ECMP routing** — The default hash is `MD5(src:dst)`, so all flows sharing a
source-destination pair use the same spine.  Pass `--ecmp-flow-id` to include
the flow ID in the hash for per-flow path variation.

**Drop tracking** — Drops at the source uplink are counted against
`flow.packets_dropped`.  Drops at intermediate hops (spine or dst-leaf queues)
are also tracked per-flow so you can see where congestion occurs.

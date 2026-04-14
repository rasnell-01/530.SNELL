# Simulating Incast, ECMP Imbalance, and Queueing in a Leaf-Spine Data Center

**Ryan A. Snell**

---

## Abstract

This report covers the simulation of a leaf-spine data center network. The simulation models three distinct traffic patterns — uniform random, incast, and hotspot — and measures the impact each has on queue depth, packet drops, and flow completion time. The results confirm what networking theory predicts: incast will crush a receiver's downlink regardless of how well the fabric is built, ECMP hash collisions will leave paths idle while others are overloaded, and larger buffers eliminate drops at the price of much worse latency. Each experiment is backed by real output from the simulator.

---

## Why a Leaf-Spine Topology Helps Bisection Bandwidth

A classical tree network — access, aggregation, core — forces all cross-rack traffic to climb up to the same oversubscribed aggregation and core links. Bisection bandwidth, the bandwidth available when you split the network in half, ends up being a fraction of the total edge link capacity. That bottleneck is unavoidable in a tree design no matter how fast the individual links are.

A leaf-spine fabric solves this directly. Every leaf switch connects to every spine switch and every spine connects back to every leaf. That gives you k equal-cost paths between any two racks, where k is the number of spines. As long as traffic gets spread across those paths, bisection bandwidth scales linearly with the number of spines. Adding a spine switch increases capacity for every leaf pair at the same time. There is no single chokepoint the way there is in a tree. The result is a fabric where aggregate bandwidth can approach the sum of all server-facing link rates — something a tree design simply cannot deliver.

---

## Why Incast Still Occurs in a Highly Connected Fabric

Incast is a convergence problem, not a connectivity problem. It does not matter how many spines the fabric has. Every flow destined for the same receiver still has to exit the fabric through that receiver's single downlink from its leaf switch. That last hop is a serial bottleneck shared by every incoming flow, and no amount of spine switches changes that.

The simulation makes this impossible to ignore. With 16 senders each pushing 20 packets at a link capacity of 4 packets per tick, the receiver's downlink gets hit with up to 16 packets per tick while it can only service 4. The queue fills to its 32-packet limit almost instantly and 58.8% of all packets are dropped. Adding more spines would not help here at all. The congestion is at the leaf-to-server downlink, not in the spine fabric. The only real fixes are receiver-side pacing, synchronized send windows, or application-level staggering of when workers report results.

---

## Why ECMP Does Not Guarantee Perfect Load Balance

ECMP selects a path by hashing the packet header — typically the source IP, destination IP, and ports. That hash is deterministic and has no memory of how much traffic has already been sent on each path. Two heavy flows that happen to hash to the same spine will both pile onto that spine even when the other spines are completely idle. The switch has no way to know and no mechanism to correct it.

There are two ways this goes wrong. First, when the number of active flows is small relative to the number of paths, hash collisions become likely. In Experiment 2, with 40 flows across 4 spines, the coefficient of variation of spine utilization sat at 0.25 — nowhere near the 0.0 that perfect balance would produce. Second, if one enormous flow and several small flows all hash to the same spine, that flow dominates the shared path while the other spines go unused.

Experiment 3 showed this clearly with 60 uniform flows. The busiest spine-outbound link hit 0.094 utilization while several spine links carried zero packets. The ratio of max to min utilization was effectively unbounded. ECMP works well at large scale with many flows, but it offers no guarantees, especially when the flow count is low or flow sizes are highly skewed.

---

## Why Tail Latency Matters More Than Average Latency

In a distributed system, a single user request typically fans out to dozens or hundreds of backend servers in parallel. The overall response time is determined by the slowest sub-request, not the average. If a request fans out to 100 servers and one of them is slow, the user waits on that one server. The average response time across all 100 servers is irrelevant to the user experience.

The math here is straightforward. If each sub-request has a 99th-percentile latency of 200ms and you fan out to 100 servers, nearly every user-facing request will take at least 200ms. Average latency of 20ms does nothing to help that user.

Network queuing is one of the biggest drivers of tail latency. Short flows caught behind a large flow in the same queue experience latency orders of magnitude above what their transfer size would otherwise require. Those delays show up in the tail. In the incast simulation, the average FCT was 28 ticks but the maximum was 29 ticks. With identical flows that gap looks small. In production with mixed flow sizes, that tail gets dramatically worse. Optimizing for average latency while ignoring the tail means the majority of user experiences are being measured incorrectly.

---

## How Application Communication Patterns Shape Network Behavior

The application communication pattern determines which links get stressed, how queues build, and how many packets get dropped. The fabric does not choose — it just moves packets wherever it is told.

Uniform random traffic distributes load broadly across the fabric. With a reasonable flow count, ECMP spreads traffic across multiple spines, queues stay shallow, and everything completes without drops. The uniform simulation produced 0 drops, an average queue length of 0.031 packets, and 100% delivery. The fabric was not stressed at all.

Incast, the all-to-one pattern used in MapReduce, distributed query aggregation, and scatter-gather storage, is the adversarial case. Many workers completing a phase simultaneously and reporting to a single coordinator produces exactly the burst that overflows the coordinator's downlink queue. Experiment 1 shows the crossover: at 1 to 4 senders there are zero drops, but at 8 senders the drop rate jumps to 32.5% and keeps climbing to 77.5% at 32 senders.

Hotspot traffic mimics social media hot content, popular database partitions, or heavily accessed storage nodes. As the fraction of flows directed at hot receivers increases from 0% to 100%, drop rate grows from 0% to 56.2% and average flow completion time rises from 21.7 to 28.5 ticks. The rest of the fabric sits idle while the hot receivers' downlinks are saturated.

The practical takeaway is that networks need to be provisioned for the worst-case communication pattern the applications will produce, not the average. A fabric that handles uniform traffic perfectly can fail completely under incast. Understanding what the application actually does during peak load is not optional — it is the only way to design a network that works when it has to.

---

## Experiment Results

### Experiment 1: Incast — FCT vs. Number of Simultaneous Senders

This experiment swept the incast sender count from 1 to 32 with a queue capacity of 32, link bandwidth of 4 packets per tick, and 20-packet flows.
```mermaid
| Senders | Avg FCT | Max FCT | Drop % |
|--------:|--------:|--------:|-------:|
| 1       | 22.0    | 22      | 0.0%   |
| 4       | 22.0    | 22      | 0.0%   |
| 8       | 29.0    | 29      | 32.5%  |
| 16      | 28.0    | 29      | 58.8%  |
| 32      | 27.0    | 27      | 77.5%  |
```
Up to 4 senders there are zero drops and FCT is stable. At 8 senders the aggregate injection rate exceeds the downlink capacity and the queue starts overflowing. FCT rises and then actually stabilizes at higher sender counts because most packets are being dropped before they can contribute to delivery. At 32 senders, over three quarters of all packets are simply lost.

---

### Experiment 2: ECMP Balance Under Uniform Traffic

This experiment compared two ECMP hash functions on 40 flows across 4 spines: hash(src, dst) only versus hash(src, dst, flow_id).

With hash(src, dst), the spine utilization distribution had a coefficient of variation of 0.25. Adding the flow ID brought it to 0.26. Neither approach produced balanced load. With only 40 flows, hash collisions are common enough that some spines carry significantly more traffic than others regardless of the hash function used. The per-flow hash variant helps at larger flow counts, but it is not a complete solution.

---

### Experiment 3: ECMP Overload on Spine Links

Running 60 uniform flows confirmed that ECMP can leave individual spine links heavily loaded while others carry nothing. The busiest spine-outbound link hit 0.094 utilization and the lightest carried zero packets. The max-to-min utilization ratio was effectively unbounded because unused paths always exist with this flow count. ECMP with 60 flows and 4 spines is simply not enough volume to smooth out the hash distribution.

---

### Experiment 4: Effect of Queue Buffer Size on Incast

This experiment swept queue capacity from 4 to 256 packets with 16 incast senders.

```mermaid


| Queue Size | Avg FCT | Drop % |
|-----------:|--------:|-------:|
| 4          | 21.0    | 73.8%  |
| 32         | 28.0    | 58.8%  |
| 128        | 52.0    | 28.7%  |
| 256        | 75.8    | 0.0%   |
```

The tradeoff is direct. Small queues drop the majority of packets but complete surviving flows quickly. Large queues eliminate drops entirely but impose nearly 4x longer completion times because packets sit in a deep queue waiting to be serviced. A queue of 256 is large enough to absorb the full burst from 16 senders but it turns a 22-tick flow into a 76-tick flow. This is bufferbloat in action — bigger buffers do not solve congestion, they just delay it.

---

### Experiment 5: Hotspot Traffic Concentration

This experiment increased the fraction of flows directed at 2 hot receivers from 0% to 100%.

```mermaid
| Hot Fraction | Avg FCT | Drop % |
|-------------:|--------:|-------:|
| 0.00         | 21.7    | 0.0%   |
| 0.30         | 23.0    | 6.2%   |
| 0.70         | 25.4    | 32.7%  |
| 1.00         | 28.5    | 56.2%  |
```
Drop rate climbs monotonically with concentration. Even moderate skew at 30% produces measurable packet loss. At 100% concentration the result is nearly identical to an incast scenario — more than half of all packets are dropped and many flows never complete. The hot receivers' downlinks saturate while the rest of the fabric is barely used. This confirms that the problem is always at the last hop to the destination, not in the spine.

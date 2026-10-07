# Capacity-limited dispatch of three production units against a demand floor.

A grid operator, a logistics planner or a plant scheduler has several units
that can each supply up to a fixed capacity, at a cost that rises with
output, and must together meet a demand. The question is how much each unit
should produce: the cheapest allocation that still meets demand, if demand
can be met at all. The solve runs on a server-side worker, where it is one of
many dispatch problems in a batch.

Run:

```text
cargo run --manifest-path examples/cluster-capacity-dispatch/Cargo.toml
```

See also: the root [`README`](../../README.md) and the
[cluster user guide](../../docs/src/cluster-user-guide.md).

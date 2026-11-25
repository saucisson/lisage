# ALIGN-3 — Share of the observer dispatch on the build path

Measurement only, no refactoring. Question (from the campaign-3 verdict): the
residual vs OxiDD grows with n — how much of the build time is the observer
dispatch (`Box<dyn Observer>::on_event` on the intern path) itself?

## Protocol

New runner `crates/lisage-bench/src/bin/observer_share.rs`, one process per
run, counting global allocator (MEM-18 pattern), same charge as the PERF
publications (alea-lcg, bottom-up Shannon expansion, 2^n − 1 intern calls).
Two modes:

- `noop` — `NoOpObserver`, the default wired into `profile_intern` (the
  neutral baseline);
- `counting` — a wired observer tallying every event with atomic counters
  (no allocation, no I/O): the lightest real consumer of the trace, standing
  in for Liseuse's serializer without its I/O cost.

5 direct release runs per mode per charge, median reported (min/max in
parentheses). Same machine, same session, 2026-11-20. Under the nextest
harness the runner is ~10x slower (test-process contention on this 4-core
WSL2 box); the direct `cargo run --release` numbers are the reference.

Reproduce:

```
cargo build --release -p lisage-bench --bin observer_share
for mode in noop counting; do for i in 1 2 3 4 5; do
  cargo run --release -q -p lisage-bench --bin observer_share -- $mode 20
  cargo run --release -q -p lisage-bench --bin observer_share -- $mode 22
done; done
```

## Results (build phase only, median of 5)

| charge | mode | build time (ms) | allocs build | bytes build | peak RSS (KiB) |
|---|---|---|---|---|---|
| alea-lcg-20 | noop | 78.364 (74.488–81.673) | 1 048 803 | 45 159 844 | 12 412 |
| alea-lcg-20 | counting | 84.348 (82.053–85.867) | 1 048 803 | 45 159 844 | 12 408 |
| alea-lcg-22 | noop | 355.203 (348.871–378.388) | — | — | — |
| alea-lcg-22 | counting | 379.075 (368.474–408.130) | — | — | — |

**Observer share of the build time: 7.64 % at n=20 (Δ 5.984 ms), 6.72 % at
n=22 (Δ 23.872 ms).** Allocations, transient bytes and RSS are identical
across modes; node counts identical (106 760 at n=20, 325 824 at n=22 — C1,
differential green).

Event tallies (counting mode, n=20): 2 097 151 events = 106 760 NodeCreated
+ 1 654 959 NodeShared + 335 432 ReductionApplied (a default reduction often
precedes a share, so the total is ~2 events per intern call; 1 048 575 intern
calls). Per-event cost: 5.984 ms / 2 097 151 ≈ **2.85 ns/event** at n=20,
23.872 ms / 8 388 607 ≈ **2.84 ns/event** at n=22 — a constant per-event
dispatch cost, not a per-node cost.

## Conclusion (written, as the increment requires)

**The observer share is weak: ~7 % of the build time** (6.7–7.6 % across
n=20/22) for the lightest real consumer of the trace, at a constant
~2.8 ns/event. It is not the driver of the residual vs OxiDD (1.39x at n=20,
2.04x at n=24 in campaign-3). A heavier observer (serialization + I/O) would
pay more, but that cost is opt-in instrumentation, not the default path.

→ **Note and close.** The structural answer remains D4-1 (compilable,
deactivatable instrumentation gate, issue #168), already planned; no new
increment is drafted. ALIGN-3 is a measurement, no code path changed.

## Gates

- C1: node/intern counts identical across modes; `make check` green
  (371 passed, 11 skipped on this commit), differential tests green.
- MEM-18 thresholds: green, untouched.
- Structural test `crates/lisage-bench/tests/observer_share.rs` pins the
  counters (identical nodes/allocs across modes, one NodeCreated per node,
  events >= intern calls); times are published here, never pinned in tests.

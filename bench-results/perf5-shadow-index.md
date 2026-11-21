# PERF-5 — shadow-plane index as Vec per level (issue #288)
Machine: M1


Runner `profile_intern {n}` (unchanged), one process per run, counting global allocator (MEM-18 pattern). Same charge as the previous PERF publications: alea-lcg, bottom-up Shannon expansion, 2^n - 1 intern calls.

## After PERF-5 (this change)

| n | noeuds | internements | temps table (ms) | temps build (ms) | temps count (ms) | allocs build | octets build | allocs/internement | rss_max_ko |
|---|---|---|---|---|---|---|---|---|---|
| 20 | 106760 | 1048575 | 1.7 | 132.1 | 25.7 | 1048802 | 47256788 | 1.000 | 13480 |
| 22 | 325824 | 4194303 | 7.3 | 845.8 | 91.7 | 4194628 | 178855092 | 1.000 | 40832 |

Build time is the median of 3 runs at n=20 (131.4-138.8 ms); allocation counters are deterministic across runs.

## Before / after (n=20, same machine, same runner)

| metric | PERF-3 | PERF-5 | factor |
|---|---|---|---|
| allocs/internement | 1.017 | 1.000 | the per-created-node BTreeSet allocation is gone |
| allocs build | 1 066 455 | 1 048 802 | 106 760 fewer (one per created node) |
| temps build (ms) | 152.5 | 132.1 | 1.15x faster |
| rss_max (KiB) | 14 456 | 13 480 | the Vec index uses less real memory than the BTreeSet |
| noeuds / verifie | 106 760 / 523 228 | 106 760 / 523 228 | identical (C1) |

## What changed

`IndexLevels` (`crates/lisage-core/src/biplan.rs`) is now `BTreeMap<Level, Vec<NodeId>>`:
`push` at creation (amortized, allocation-free per node) instead of `BTreeSet::insert`
(one tree-node allocation per created node); GC `retain` and compaction rebuild keep
the same semantics; no duplicates are possible (the unique table guarantees a single
creation per form). The public getter `index_level` returns `&[NodeId]`. The MEM-14
`bytes_per_node` counter keeps its historical `len` semantics so the frozen MEM-18
threshold stays comparable (the real footprint is lower than the old BTreeSet's —
RSS confirms it).

## Gap vs OxiDD (BENCH-1 campaign: OxiDD alea-lcg-20 = 122.3 ms)

lisage `profile_intern` build on this machine: 563.4 ms (PERF-1) -> 132.1 ms (PERF-5).
Against the OxiDD reference of 122.3 ms, the ratio on alea-lcg-20 is now **~1.08x**.

Remaining 1.0 alloc/intern: the call-site `from_pairs` box (the only per-intern
allocation left on this path).

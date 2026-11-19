# PERF-3 — fast deterministic hash + no dense box for inline classes (issue #284)

Runner `profile_intern {n}` (unchanged), one process per run, counting global allocator (MEM-18 pattern). Same charge as `perf1-profile-intern.md` / `perf2-allocs-intern.md`: alea-lcg, bottom-up Shannon expansion, 2^n - 1 intern calls.

## After PERF-3 (this change)

| n | noeuds | internements | temps table (ms) | temps build (ms) | temps count (ms) | allocs build | octets build | allocs/internement | rss_max_ko |
|---|---|---|---|---|---|---|---|---|---|
| 20 | 106760 | 1048575 | 1.7 | 152.5 | 25.3 | 1066455 | 47445764 | 1.017 | 14456 |
| 22 | 325824 | 4194303 | 6.6 | 864.6 | 87.2 | 4248757 | 180016076 | 1.013 | 42740 |

Build time is the median of 3 runs at n=20 (148.0-161.4 ms); allocation counters are deterministic across runs.

## Progression across the PERF milestone (n=20, same machine, same runner)

| metric | PERF-1 baseline | PERF-2 | PERF-3 | total |
|---|---|---|---|---|
| allocs/internement | 8.479 | 1.697 | 1.017 | 8.3x fewer |
| allocs build | 8 890 949 | 1 779 598 | 1 066 455 | 8.3x fewer |
| octets build (transient) | 468 087 764 | 81 676 628 | 47 445 764 | 9.9x less |
| temps build (ms) | 563.4 | 214.4 | 152.5 | 3.7x faster |
| noeuds / verifie | 106 760 / 523 228 | 106 760 / 523 228 | 106 760 / 523 228 | identical (C1) |

## What PERF-3 changed

1. `hash_cle` (`crates/lisage-core/src/unique.rs`): `DefaultHasher` (SipHash-1-3) replaced by `FastHasher`, an FxHash-style rotate-xor-multiply hasher over 64-bit words with a fixed seed (golden ratio) and little-endian reads — deterministic across runs and machines (MEM-18 seeded workload). The unique table uses the hash only for bucket placement (equality stays semantic via `same_form`), so any deterministic hasher is correct. Build 214 -> 162 ms at n=20.
2. `Arena::intern` (`crates/lisage-core/src/arena.rs`): the MEM-12 dense-family allocation is skipped when the node is routed to an exact inline class (its arcs are stored inline there, the boxed run would be dropped). The decision is deterministic in (level, keys, content, layout), so the hash-consing key stays canonical and C1 holds; `resolve` still re-applies the dense family on rebuild. allocs/intern 1.697 -> 1.017, build 162 -> 152 ms.

Remaining ~1.0 alloc/intern: the call-site `from_pairs` box and amortized arena slab growth.

## Gap vs OxiDD (BENCH-1 campaign 20261118-campaign-2: OxiDD alea-lcg-20 = 122.3 ms)

lisage `profile_intern` build on this machine: 563.4 ms (PERF-1) -> 152.5 ms (PERF-3). Against the OxiDD reference of 122.3 ms, the ratio on alea-lcg-20 goes from ~4.4x to ~1.2x.

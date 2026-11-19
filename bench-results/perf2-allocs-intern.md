# PERF-2 — allocations per intern cut on the alea-lcg intern path (issue #282)

Runner `profile_intern {n}` (PERF-1 runner, unchanged), one process per run, counting global allocator (MEM-18 pattern). Same charge as `perf1-profile-intern.md`: alea-lcg, bottom-up Shannon expansion, 2^n - 1 intern calls.

## After PERF-2 (this change)

| n | noeuds | internements | temps table (ms) | temps build (ms) | temps count (ms) | allocs build | octets build | allocs/internement | rss_max_ko |
|---|---|---|---|---|---|---|---|---|---|
| 20 | 106760 | 1048575 | 1.6 | 214.4 | 25.3 | 1779598 | 81676628 | 1.697 | 14424 |
| 22 | 325824 | 4194303 | 6.4 | 1158.6 | 89.8 | 7088820 | 316339100 | 1.690 | 42680 |

Build time is the median of 3 runs (213.4-214.8 ms at n=20); allocation counters are deterministic across runs.

## Before / after (n=20, same machine, same runner)

| metric | PERF-1 baseline | PERF-2 | factor |
|---|---|---|---|
| allocs/internement | 8.479 | 1.697 | 5.0x fewer |
| allocs build | 8 890 949 | 1 779 598 | 5.0x fewer |
| octets build (transient) | 468 087 764 | 81 676 628 | 5.7x less |
| temps build (ms) | 563.4 | 214.4 | 2.6x faster |
| noeuds / verifie | 106 760 / 523 228 | 106 760 / 523 228 | identical (C1) |

## What was cut (per intern call)

1. `ArcRun::from_pairs` built a `BTreeMap` (one heap node per pair + the boxed slice): now Vec with exact capacity from the size hint + sort + dedup last-wins — 1 allocation for the common duplicate-free case.
2. `ArcRun::iter()` returned `Box<dyn DoubleEndedIterator>` — one heap allocation per call, and the intern path calls it several times per intern (unique-table probes via `same_form`, `InlineEntry::from_form`, `collapse_arcs`, `narrowable`): now a concrete allocation-free `ArcRunIter`.
3. `Arena::form_normalisee` rebuilt the run even when every key was already in normal form (BDD identity): fast path keeps the run when it is already canonical (sorted family, strictly increasing keys, normal keys).
4. `shannon` copied two half-tables (`Vec<bool>` x2) per level: now a strided window `(table, offset, stride)`, no copy.

Remaining ~1.7 allocs/intern: the `from_pairs` box at the call site, the dense-family box of `choose_dense_family` (MEM-12, kept), and amortized arena slab growth.

## Gap vs OxiDD (BENCH-1 campaign 20261118-campaign-2: OxiDD alea-lcg-20 = 122.3 ms)

lisage build 533.9 ms -> 214.4 ms: the lisage/OxiDD time ratio on alea-lcg-20 goes from ~4.4x to ~1.8x.

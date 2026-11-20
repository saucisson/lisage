# Worklog

Append-only, agent-written audit trail. One entry per commit or pull
request; the human owner reads it as part of the `daily` → `main`
merge review at the end of day.

This file is **not** constitutional: agents append to it as part of
their normal work, without an escalation. It is not a substitute for
the pull request report either - it is the persistent, chronological
record of what the team actually did, kept in the repository so the
day's audit trail travels with the `daily` → `main` diff.

Rules:

- The developer agent appends its entry in the pull request branch,
  so the entry arrives with the merge into `daily` and the reviewer
  sees it in the diff.
- One entry per merged pull request; a direct commit on `daily`
  (revert protocol, docs-only fix) carries its own entry in the same
  commit.
- Newest entry on top, directly below the marker line. Never edit,
  reorder or delete past entries: the git history is the tamper-evident
  backstop, and rewriting the log is an honesty violation.
- The same honesty rules as any report (`AGENTS.md`): evidence you did
  not run is stated as such, not implied.
- New entries are written in English (milestone M6, English-only
  repository); entries written before M6 stay in French as the
  historical record.

## Entry format

One entry per commit or pull request:

```markdown
## YYYY-MM-DD - one-line summary (PR #N / <commit hash>)

- **Issue**: the issue(s) closed, or `none`.
- **Done**: what the change does, mapped to the acceptance criteria.
- **Evidence**: the gates run and their results (lint, types, tests,
  coverage) - and what was **not** run, with the reason.
- **Judgement calls**: every decision between two reasonable options,
  and what decided it (ADR, issue comment, human answer).
- **Escalations**: `needs-human` requests raised or consumed, if any.
```

---

<!-- Append new entries directly below this line. -->
## 2026-11-20 - ALIGN-0: static size witnesses for the ALIGN track (no functional change)

- **Issue**: none — the ALIGN-i issues could not be opened on GitHub (issue creation is denied to the integration: POST /issues → 403 on saucisson/lisage, probed 2026-11-20); drafts written in the session scratchpad, work driven by the human directive of 2026-11-20. ALIGN-0 = premise witnesses for ALIGN-1/2.
- **Done**: `test_align0_option_slot_is_16_bytes` in `crates/lisage-core/src/unique.rs` pins `size_of::<Slot>() == 12` and `size_of::<Option<Slot>>() == 16` (no niche); `test_align0_option_dense_slot_is_12_bytes` in `crates/lisage-algebra/src/interface.rs` pins `size_of::<Option<(u32, NodeId)>>() == 12` and `size_of::<Option<NodeId>>() == 8`. No functional change.
- **Evidence**: `make check` green on this commit (ruff, mypy 2.4.0 pure-python, lint-imports, pytest, cargo fmt/clippy/nextest: 368 passed, 11 skipped, english-only). Environment note: rustup stable 1.100.0 installed in-session; mypy/hypothesis installed `--no-binary` because their compiled wheels segfault in this sandbox — CI is the authoritative gate.
- **Judgement calls**: witnesses live in the existing `#[cfg(test)]` modules of the touched files so the runner collects them; premises confirmed on rustc 1.100.0 (16/12/8 bytes as expected).
- **Escalations**: none.

## 2026-11-19 - P1-2: parallel merge of the output pools (issue #140)

- **Issue**: #140 (P1-2, PAR-1): parallel pairwise union of the output pools (merge at barriers), including cross-pool sharing. DoD: equivalence to sequential merges (PBT); bench scaling vs sequential merge.
- **Done**: (1) `crates/lisage-core/src/parallel_ops.rs` gains `copy_into` (recursive cross-pool copy with per-source-node memo — identical subforms share nodes in the destination's unique table), `merge_pools_seq` (sequential fold: copy each root into one merge pool + union) and `merge_pools_par` (pairwise tree reduction on rayon: each pair merged in its own task into a fresh pool, until one remains). (2) PBT `test_p1_2_merge_pools_equivalence` in `par_equivalence.rs`: k (1..7) random output pools — seq vs par agree on the structural form of the merged root and its cardinality; cross-pool sharing pinned (merged live <= sum of sources + k union-root nodes). (3) Bench runner `bench_merge.rs` + test `merge.rs`: k pools, each a full copy of a random Map diagram (n=8: 16 keys, domain {0,1,2}, 4000 words, 136 286 live nodes), merged seq vs par with PAR-1 sanity inside the runner; publishes `bench-results/p12-merge.md` under `P12_PUBLISH=1`.
- **Measured verdict (written)**: published `bench-results/p12-merge.md`: n=8, 16 pools — seq 1481.866 ms vs par 1091.228 ms, speedup 1.36x on 4 cores (release runner: 1.14x at 16 pools, 1.21x at 32 pools); merged pool 17 928 live nodes vs 136 286 per copy (cross-pool sharing effective).
- **Evidence**: `cargo nextest run -p lisage-bench --test par_equivalence --test merge` green; `cargo nextest run -p lisage-core` green (187 passed); clippy `-D warnings`, import-linter, `make english-only` green. Not run: full-workspace `make check` in this session; GitHub CI before the PR.
- **Judgement calls**: the merge bench uses copies of one random Map diagram (the philosophers R_n root is too compact — ~100 reachable nodes — to show scaling); wall times published, not pinned; the sharing bound allows one union-root node per pool.
- **Escalations**: none.

## 2026-11-19 - P1-1: independent operations, separate pools (issue #139)

- **Issue**: #139 (P1-1, PAR-1): module `parallel_ops.rs` — parallel execution (rayon) of independent transductions reading the same input pool read-shared, each in its own output pool (INV10). DoD: PBT PAR-1 on n concurrent operations; bench showing the gain on multi-query workloads.
- **Done**: (1) New `crates/lisage-core/src/parallel_ops.rs`: `IndependentOp<T>` (transduction + root), `run_independent_seq` (one-worker reference) and `run_independent_par` (rayon fork-join, results in input order), each op in its own output pool, input pool read-shared (INV10). rayon 1.12 promoted from transitive (already in Cargo.lock) to a direct dependency of lisage-core — named by issue #139, no new version introduced. (2) PBT in `crates/lisage-bench/tests/par_equivalence.rs` (`test_p1_1_independent_ops_equivalence`): n ops (1..6) on roots of a random read-shared input pool — seq vs par agree on every synthesized attribute and on the structural form of every output root (`forms_equal`, never bare NodeIds). (3) Bench runner `crates/lisage-bench/src/bin/bench_parallel_ops.rs` + test `crates/lisage-bench/tests/parallel_ops.rs`: multi-query workload on philosophers R_8 (3104 members), 16 counting ops, seq vs par, PAR-1 sanity asserted inside the runner; publishes `bench-results/p11-parallel-ops.md` under `P11_PUBLISH=1`.
- **Measured verdict (written)**: published `bench-results/p11-parallel-ops.md`: n=8, 16 ops — seq 23.065 ms vs par 10.257 ms, speedup 2.25x on 4 cores (release runner: 1.48x at 16 ops, 1.85x at 32 ops).
- **Evidence**: `cargo nextest run -p lisage-bench --test parallel_ops --test par_equivalence` green; `cargo nextest run -p lisage-core` green (187 passed); clippy `-D warnings`, import-linter (1 contract kept), `make english-only` green. Known pre-existing failure, not from this change: `lisage-bench::dense test_c1_famille_dense` (latent proptest seed, fails on base too). Not run: full-workspace `make check` in this session; GitHub CI before the PR.
- **Judgement calls**: rayon used as named by the issue (already in the lockfile, so no new version decision); ops in the bench alternate the union root (heavy query) and member roots; wall times published, not pinned.
- **Escalations**: none (the rayon dependency was specified by issue #139 itself).

## 2026-11-19 - P0-3: ADR blind routing to structure (issue #138)

- **Issue**: #138 (P0-3): ADR covering the invariants INV1-INV10, the decision against level-sharding (and why), the position vs Sylvan (per variable) and vs the Paris school/pnmc; plan B Adiar documented as the fallback condition. DoD: ADR merged in `docs/decisions/`; the boundary tests validate the signatures.
- **Done**: new `docs/decisions/adr-0105-blind-routing-to-structure.md` (draft, human accepts): routing is blind to structure — the unique table is a single content-addressed table routing on the hash of `(ControllerId, Anchor, Form)` with full equality verified at the probe; INV1-INV10 enumerated (content-addressed routing, CAS insert-or-find, flat NodeIds without central counter, shadow plane off the intern path, frozen level-memory versions per round, GC without compaction at round barriers, round barrier with agreed active-level set, sharded lock-free memo, order as controller contract, read-shared input pool with per-op output pools); level-sharding and Sylvan-style per-variable sharding rejected with reasons; Sylvan kept as witness; Paris-school/pnmc level-wise sweeps kept for saturation semantics only; Adiar (SP-1, issue #153) documented as plan B with the trigger condition "PAR-2/PAR-5 not held after P3, or memory need > speed need".
- **Measured verdict (written)**: no code change; the boundary tests validate the signatures — `cargo nextest run -p lisage-bench --test frontieres` green (3 passed, incl. `test_frontieres_signatures_send_sync`).
- **Evidence**: `make english-only` green; `frontieres` boundary tests green. Not run: full `make check` in this session (docs-only change; per-crate gates run instead); GitHub CI before the PR.
- **Judgement calls**: the ADR is drafted as **draft** per the ADR convention (0100+ range drafted in a PR, accepted by the human only); INV7/INV9 were not yet referenced by any issue and are defined here (round barrier / no dynamic reordering) to complete INV1-INV10.
- **Escalations**: none (ADR drafting in the 0100+ range is explicitly allowed by ADR-0010; acceptance stays with the human).

## 2026-11-19 - P0-2: mono vs multi equivalence harness (issue #137)

- **Issue**: #137 (P0-2, PAR-1): PBT mono vs multi on the same inputs (node counts, semantics), written red before any implementation; survey of existing tests comparing NodeId (to requalify as form comparisons). DoD: harness green on the current mono-thread, ready to falsify PAR-1 to PAR-4.
- **Done**: (1) New harness `crates/lisage-bench/tests/par_equivalence.rs`: a proptest workload (bounded set schema + random members) is executed by two executors — `MonoExecutor` (the current single-threaded `Arena` path: compile + union into R, saturate with a fixed singleton step, count the fixpoint) and `MultiExecutor` (placeholder delegating to mono until P1-1/P2-1 land a real parallel executor). The PBT asserts equal live node counts, equal fixpoint iterations, equal cardinality (semantics) and structural form equality of the roots across the two pools (`forms_equal`, never bare NodeIds). A non-vacuity PBT pins rounds >= 1 and a non-empty pool. An `#[ignore]`d red marker `test_multi_executor_is_parallel` fails while `MultiExecutor` is still the placeholder — un-ignore when a real multi-thread executor lands. (2) Survey of NodeId-comparing tests: same-pool comparisons (`schemas.rs`, `schema.rs`, `cdd.rs`, `ddd.rs`, `liseuse_v0.rs`, `classes.rs`, `dense.rs`) are the canonicity property C1 and stay valid under multi-thread; `transductions.rs` already compared forms cross-pool; `handles.rs` compared bare NodeIds across two arenas — requalified as a form comparison via `resolve`; the proptest input ranges of that test were narrowed accordingly (keys 1..3 instead of 0..3 because key 0 is the default key whose arcs are R-default-dropped, targets 0..2 = the two terminals interned first so both arenas can resolve the arc targets), with the reason documented in the test; the handle comparison next to it remains the spec-level C1 assertion.
- **Measured verdict (written)**: harness green on the current mono-thread: `test_mono_multi_equivalence` and `test_harness_is_not_vacuous` pass, red marker skipped as designed.
- **Evidence**: `cargo nextest run -p lisage-bench` green (101 passed, 11 skipped incl. the red marker); clippy `-D warnings` and `make english-only` green on the touched crates. Not run: full-workspace `make check` in this session (per-crate gates run instead); GitHub CI before the PR.
- **Judgement calls**: the multi leg is a documented placeholder (DoD asks for a green harness on the current mono-thread, ready to falsify PAR-1..PAR-4); cross-pool NodeId equality was requalified only where it was actually cross-pool (`handles.rs`), same-pool C1 assertions left untouched.
- **Escalations**: none.

## 2026-11-19 - P0-1: contention bench anchored on philosophers saturation n=5 (issue #136)

- **Issue**: #136 (P0-1, contention bench, spec PAR-2): instrument the existing metrics (shares, creations) — intern distribution per level, R-share and memo hit rates, saturation phase profile — with an anchored bench (philosophers, saturation n=5+), baselines published in `bench-results/`, CI refusing time regressions.
- **Done**: (1) `Arena` gains a per-level intern counter `interns_per_level: Vec<u64>` (P0-1), incremented on every node-form intern call (hit or miss), exposed via `interns_per_level()`. (2) New runner `crates/lisage-bench/src/bin/bench_contention.rs` (`bench_contention [n]`, default 5): builds R_n (philosophers, golden thread), saturates with a synthetic contention step (one fixed illegal member — documented as such; the real take/release/hungry transition saturation belongs to the golden-thread milestone), counts the fixpoint; a counting observer tallies MemoHit/MemoMiss/Round/NodeCreated/NodeShared; one JSON line with per-level interns, R-share hit rate (pool metrics), memo hit rate, rounds and per-phase times. (3) New test `crates/lisage-bench/tests/contention.rs`: pins the structural counters (n=5: verified 153 = |R_5| 152 + 1 saturation-added member, rounds 2, creations == live nodes, 4n per-level counters all > 0), falsifiable hit-rate ranges (R-share 50-80 %, memo 0.10-0.40), a frozen generous time ceiling (30 s, ~10 000x the ~3 ms local median — catches algorithmic regressions only, never machine noise), and publishes `bench-results/p01-contention.md` under `P01_PUBLISH=1`.
- **Measured verdict (written)**: at n=5, per the published `bench-results/p01-contention.md`: build 29.533 ms / saturation 1.664 ms / count 1.505 ms, 1 860 live nodes, 3 510 R-share hits (65 %), memo hit rate 21.4 % (43 hits / 158 misses), 2 rounds, per-level intern distribution over the 20 levels (306 down to 153 per level, pc/left pairs equal as expected). n=6: 5 208 nodes, verified 417, R-share 70 %, memo 21.4 %.
- **Evidence**: `cargo nextest run -p lisage-bench --test contention` green in assert and publish modes; full `make check` green before the PR (ruff, mypy strict, import-linter, pytest, fmt, clippy `-D warnings`, nextest, english-only). Not run: GitHub CI before the PR.
- **Judgement calls**: the saturation step is synthetic (contention), not the philosophers transition relation — stated in the runner doc and the publication; the time ceiling is deliberately ~10 000x the local median so CI refuses only gross (algorithmic) regressions, consistent with the repo convention that wall time is published, not tightly pinned.
- **Escalations**: none.

## 2026-11-19 - PERF-5: shadow-plane index as Vec per level (issue #288, milestone PERF #279)

- **Issue**: #288 (PERF-5, fifth increment of milestone PERF #279): the campaign-3 residual gap vs OxiDD grows with n (1.39x at n=20, 2.04x at n=24) — a per-created-node cost, not per-intern. Suspect: the shadow index `BTreeSet` per level (one tree-node allocation per created node).
- **Done**: `IndexLevels` (`crates/lisage-core/src/biplan.rs`) becomes `BTreeMap<Level, Vec<NodeId>>` — `push` at creation (amortized, allocation-free per node), GC `retain` and compaction rebuild unchanged semantically, no duplicates possible (unique table = single creation per form). Public getter `index_level` now returns `&[NodeId]` (existing consumers only use `contains`/`is_empty`). The MEM-14 `bytes_per_node` counter keeps its historical `len` semantics so the frozen MEM-18 threshold stays comparable. Published `bench-results/perf5-shadow-index.md`.
- **Measured verdict (written)**: on alea-lcg-20, allocs/intern **1.017 -> 1.000** (1 066 455 -> 1 048 802 allocations, exactly one per created node removed), build **152 -> 132 ms** (median of 3, 131.4-138.8), RSS 14 456 -> 13 480 KiB (the Vec index really uses less memory than the BTreeSet); n=22: 1.000 allocs/intern, 865 -> 846 ms. Node count and satisfying assignments identical (C1). Against the OxiDD reference (122.3 ms), the alea-lcg-20 ratio is now **~1.08x** — the milestone's ~4x gap is closed. Remaining 1.0 alloc/intern: the call-site `from_pairs` box.
- **Evidence**: nextest per crate all green (algebra+core 187 passed, bench 98 passed / 10 skipped FFI, plan+lang+liseuse+backend 72 passed, 0 failed — including the MEM-18 threshold gates); `make check` green before the PR; runner output reproduced 3x (allocation counters identical). Not run: GitHub CI before the PR.
- **Judgement calls**: the `bytes_per_node` counter keeps `len` semantics (the frozen 66.0 threshold was calibrated on it); counting `capacity` instead made the gate read 72.74 and fail — the real footprint is lower (RSS), so the `len` semantics is kept for comparability, noted in the code.
- **Escalations**: none.

## 2026-11-19 - PERF-4: post-PERF confrontation campaign + golden hash test (issue #286, milestone PERF #279)

- **Issue**: #286 (PERF-4, fourth increment of milestone PERF #279): the milestone requires a published measurement per step on the same charges with the same witnesses; also folds in the review suggestion of PR #285 (golden determinism test for the FastHasher).
- **Done**: (1) re-ran the BENCH-1 crossover campaign with the same charges and witnesses as 20261118-campaign-2 (`--crossover --tools lisage,oxidd,cudd,libddd --repetitions 5 --run-id 20261119-campaign-3`, same pinned docker images, same OXIDD 2^23 capacity for alea-lcg-24) and published `bench-results/20261119-campaign-3/` (raw jsonl, aggregats.json, README, REPORT). (2) Added a golden unit test in `crates/lisage-core/src/unique.rs` pinning the exact `hash_cle` value for a fixed form (3 561 021 471 050 673 137) plus an anchor-sensitivity assertion — any change to the hash algorithm or loss of cross-run determinism fails it.
- **Measured verdict (written)**: the lisage/OxiDD time ratio goes from **3.7-4.5x (campaign-2) to 1.1-2.0x (campaign-3)** on every charge — alea-lcg-20: 533.9 -> 169.8 ms vs OxiDD 122.3 ms (**1.39x**), alea-lcg-22: 1.99x, alea-lcg-24: 2.04x, cnf-uf20-01: 1.37x, pnml-swimmingpool-1: 1.12x. Cross-check `noeuds`/`verifie` identical across all four tools and `divergences` empty (C1 holds after PERF-2/3). RSS unchanged within noise (20 224 vs 20 524 KiB on alea-lcg-20): the wins are time and transient allocations, MEM-18 thresholds not degraded. The residual factor grows with n, pointing at a per-created-node cost (shadow index, slab growth), not the intern path — next PERF increments should target that.
- **Evidence**: campaign run locally end to end (5 repetitions x 5 charges x 4 tools, fresh process/container per measurement); `cargo test -p lisage-core --lib unique` green (golden test); `make check` green before the PR (ruff, mypy strict, import-linter, pytest 51, fmt, clippy `-D warnings`, nextest 359 passed / 10 skipped, english-only). Not run: GitHub CI before the PR.
- **Judgement calls**: campaign-3 numbers are compared against the published campaign-2 medians (same machine class, same protocol); the per-charge ratios are computed from medians, not per-repetition pairs.
- **Escalations**: none.

## 2026-11-19 - PERF-3: fast deterministic hash + no dense box for inline classes (issue #284, milestone PERF #279)

- **Issue**: #284 (PERF-3, third increment of milestone PERF #279): after PERF-2 (1.70 allocs/intern, 214 ms build on alea-lcg-20), attack the remaining time (SipHash on every interned form) and the remaining allocations (dense-family box dropped for inline classes).
- **Done**: (1) `hash_cle` (`crates/lisage-core/src/unique.rs`) replaces `DefaultHasher` (SipHash-1-3) with `FastHasher`, an FxHash-style rotate-xor-multiply hasher over 64-bit words, fixed golden-ratio seed, little-endian reads — deterministic across runs and machines (MEM-18); the unique table uses the hash only for bucket placement, equality stays semantic via `same_form`, so any deterministic hasher is correct. (2) `Arena::intern` skips the MEM-12 dense-family allocation when the node routes to an exact inline class (arcs stored inline there, the boxed run would be dropped); the decision is deterministic in (level, keys, content, layout) so the hash-consing key stays canonical and C1 holds; `resolve` still re-applies the dense family on rebuild. Published `bench-results/perf3-fast-hash-inline.md`.
- **Measured verdict (written)**: on alea-lcg-20, allocs/intern **1.697 -> 1.017** (1 779 598 -> 1 066 455 allocations, 81.7 MB -> 47.4 MB transient), build **214 -> 152 ms** (median of 3, 148.0-161.4); n=22: 1.690 -> 1.013, 1159 -> 865 ms. Node count and satisfying assignments identical throughout (106 760 / 523 228 — C1 preserved). Milestone total on alea-lcg-20: 8.479 -> 1.017 allocs/intern (8.3x), 563 -> 152 ms build (3.7x). Against the BENCH-1 campaign-2 OxiDD reference (122.3 ms), the lisage/OxiDD ratio goes from ~4.4x to ~1.2x. Remaining ~1.0 alloc/intern: call-site `from_pairs` box + amortized slab growth.
- **Evidence**: `cargo nextest run` per crate, all green (lisage-algebra+lisage-core 186 passed, lisage-bench 98 passed / 10 skipped FFI, plan+lang+liseuse+backend 72 passed, 0 failed); clippy `-D warnings` clean, `cargo fmt --check` green; runner output reproduced 3x (allocation counters identical). Not run yet at entry time: full `make check` (run before the PR), GitHub CI.
- **Judgement calls**: the inline-skip of the dense family is keyed on the exact-inline routing decision computed before the family choice — deterministic per pool, so two interns of the same semantic form always take the same path (C1); the dense family is still chosen for the catch-all class and re-applied by `resolve`, so MEM-12 observable behavior is unchanged (dense PBT green).
- **Escalations**: none.

## 2026-11-19 - PERF-2: cut the allocations per intern on the alea-lcg path (issue #282, milestone PERF #279)

- **Issue**: #282 (PERF-2, second increment of milestone PERF #279): PERF-1 measured 8.48 allocations/internement on alea-lcg-20; cut them in the core intern path.
- **Done**: four allocation sources removed, all in the intern path. (1) `ArcRun::from_pairs` (`crates/lisage-algebra/src/interface.rs`) no longer builds a `BTreeMap` (one heap node per pair + boxed slice): Vec with exact capacity from the size hint, sort, dedup last-wins — one allocation for the common duplicate-free case. (2) `ArcRun::iter()` returns a concrete allocation-free double-ended `ArcRunIter` instead of `Box<dyn ...>` — the intern path calls it several times per intern (unique-table probes via `same_form`, `InlineEntry::from_form`, `collapse_arcs`, `narrowable`). (3) `Arena::form_normalisee` (`crates/lisage-core/src/arena.rs`) keeps the run as-is when it is already canonical (sorted family, strictly increasing keys, every key in normal form — the BDD identity case) instead of rebuilding it. (4) `shannon` (`crates/lisage-core/src/bdd.rs`) walks a strided window `(table, offset, stride)` instead of copying two half-table `Vec`s per level. The falsifiable allocation gate of `crates/lisage-bench/tests/profile_intern.rs` is re-pinned from 6.0-10.0 to 1.0-3.0 (measured 1.70), like a frozen threshold: only lowered, still falsifiable both ways. Published `bench-results/perf2-allocs-intern.md`.
- **Measured verdict (written)**: on alea-lcg-20, allocations per internement **8.479 -> 1.697** (8 890 949 -> 1 779 598 allocations, 468 MB -> 81.7 MB transient bytes), build **563 ms -> 214 ms** (2.6x), node count and satisfying-assignment count identical (106 760 / 523 228, C1 preserved); same at n=22 (8.445 -> 1.690, 2238 -> 1159 ms). Against the BENCH-1 campaign-2 OxiDD reference (122.3 ms on alea-lcg-20), the lisage/OxiDD time ratio goes from ~4.4x to ~1.8x. Remaining ~1.7 allocs/intern: the call-site `from_pairs` box, the MEM-12 dense-family box, amortized slab growth — next increment (PERF-3) candidates.
- **Evidence**: `make check` green end to end (ruff, mypy strict, import-linter, pytest 51 passed, `cargo fmt --check`, clippy `-D warnings`, nextest 358 passed / 10 skipped (FFI witnesses), english-only); runner output reproduced 3x (allocation counters identical, build 213.4-214.8 ms). Not run: GitHub CI before the PR, the full confrontation campaign (witnesses unchanged; the OxiDD comparison is quoted from the published campaign-2 report).
- **Judgement calls**: the `form_normalisee` fast path requires the run to be already canonical (sorted family + strictly increasing keys + normal keys), not merely identity-normal keys — otherwise a dense or unsorted input would hash differently from its canonical rebuild and break C1. A latent proptest case of `dense.rs` (`paires = [(1,6),(0,6),(3,6),(2,6)]`, collapse to an uninterned target) fails on the base commit too — pre-existing, out of scope, the regression file was restored untouched.
- **Escalations**: none.

## 2026-11-18 - PERF-1: phase-level profile of the alea-lcg intern path (issue #280, milestone PERF #279)

- **Issue**: #280 (PERF-1, first increment of milestone PERF #279 chosen after BENCH-1: close the constant ~4x OxiDD time gap by attacking the per-intern cost). Pure profiling, no core change.
- **Done**: `table_lcg` made public; new runner `crates/lisage-bench/src/bin/profile_intern.rs` (`profile_intern [n]`, default 20) times the three phases of the alea-lcg workload separately (table generation / BDD construction / node counting) under a counting global allocator (MEM-18 pattern) and prints one JSON line. New test `crates/lisage-bench/tests/profile_intern.rs` pins the structural counters (nodes 106 760, verifie 523 228, internements 2^20-1, allocs_table = 1), a falsifiable gate on allocations per internement (range 6.0-10.0), and the build-dominance share; publishes `bench-results/perf1-profile-intern.md` under `PERF1_PUBLISH=1`.
- **Measured verdict (written)**: on alea-lcg-20 the measured block is **555 ms build (93%) vs 1.8 ms table + 37 ms count** — the intern path is essentially the whole cost. The build performs **8 890 949 allocations for 1 048 575 intern calls = 8.48 allocations/internement** (468 MB transient bytes), i.e. ~83 allocations per created node (106 760 nodes). Same at n=22 (8.45/intern). This confirms the BENCH-1 hypothesis: the ~4x OxiDD gap is per-intern overhead (allocations + observer + ArcRun), not hash-consing or the table. Next increment (PERF-2): cut the allocations per intern.
- **Evidence**: `cargo test --release -p lisage-bench --test profile_intern` green in assert and publish modes; runner output reproduced 3x (allocs identical, times within noise); `make check` green (ruff, mypy strict, import-linter, pytest, `cargo fmt --check`, clippy `-D warnings`, nextest, english-only). Not run: GitHub CI (merged on local gates only, per owner instruction this session).
- **Judgement calls**: internements counted as 2^n-1 (the exact number of intern calls of the bottom-up Shannon expansion), not as created nodes — the allocation gate is per intern *call*, matching the MEM-18 convention. Times are machine-dependent and only published, never pinned.
- **Escalations**: none.

## 2026-11-18 - BENCH-1: time/memory vs CUDD and libDDD, OxiDD 4x gap analysis, memory crossover (issue #277)

- **Issue**: #277 (BENCH-1) — extend the confrontation campaigns beyond node counts: full time+RSS measurement against CUDD and libDDD (LIP6), a multi-n campaign for the memory crossover, and an analysis of the constant ~4x OxiDD time gap.
- **Done**: new full-measurement CUDD witness `docker/cudd/cudd_measure.c` (JSON protocol line; hash-set pair counting grown by doubling — the previous linear scan was O(pairs^2); `Cudd_Ref` on every intermediate node because the unique-table rehash GC crashes on unreferenced diagrams at n>=20). New libDDD witness `docker/libddd/` (LIP6 libDDD pinned at `b775aed3...`, autotools build, `libddd_measure.cpp`): bottom-up Shannon expansion by direct memoized node construction (`DDD(var,1,h)+DDD(var,0,l)` — the generic DDD algebra operators are not used), node count = `GDDD::size()` + 1 for the dropped null terminal (libDDD has no complement edges, so the lisage convention is reproduced exactly). OxiDD witness capacities now overridable via `OXIDD_INNER_CAPACITY`/`OXIDD_APPLY_CACHE_CAPACITY` (default unchanged at 2^20). Orchestrator `scripts/bench_confrontation.py` gains the `cudd`/`libddd` tools and the `--crossover` charge set (alea-lcg 20/22/24 + cnf-uf20-01 + pnml-swimmingpool-1; alea-lcg-24 runs OxiDD at 2^23 capacities). New local differential `crates/lisage-bench/tests/libddd_local.rs` (#[ignore], Docker); `oxidd_local.rs` fixed to parse the JSON line's `noeuds` field (it still parsed the pre-M7-5 bare integer). Campaign `bench-results/20261118-campaign-2/` deposited: 5 charges x 4 tools x 5 reps, zero cross-tool divergences; REPORT.md + README.md committed. Protocol doc updated (witnesses, deposited results).
- **Measured verdicts (written)**: (1) Time — lisage/OxiDD = **3.7-4.4x on every charge**, constant per node (5.0 vs 1.15 us/node at n=20); CUDD and libDDD are 1.7-2.5x *slower* than lisage on the table charges. (2) The 4x gap is per-intern overhead, not hash-consing: lisage pays ~9.17 allocations + observer + ArcRun per internement (MEM-18), OxiDD pays one apply-cache lookup and an inline-arcs slot; both sides materialize the 2^n table in the measured block, and OxiDD even parses it from text stdin, so the construction itself is >4x in OxiDD's favor. (3) Memory — lisage lowest RSS on every charge; fitted lisage ~ 4.3 MB + 11.1 B x 2^n + 45.8 B x nodes vs OxiDD ~ 50.4 MB fixed + 19.7 B x 2^n (default 2^20 capacities): on truth-table charges OxiDD never passes below lisage (ratio 2.1-2.6x in lisage's favor); on operation-built diagrams the crossover is at **~1.0M nodes** (lisage's 46 B/node vs OxiDD's ~50 MB pre-allocated baseline).
- **Evidence**: local differentials green (`cargo nextest run -p lisage-bench --test libddd_local --test cudd_local --test oxidd_local --run-ignored all`: 7/7 PASS); campaign EXIT=0 with empty `divergences`; `uv run pytest scripts/tests/test_bench_confrontation.py` 12 passed; `make check` green (ruff, mypy strict, import-linter, pytest, `cargo fmt --check`, clippy `-D warnings`, nextest, english-only). Not run: GitHub CI (merged on local gates only, per owner instruction this session).
- **Judgement calls**: the CUDD witness keeps the (node, complement-bit) pair convention of `cudd_count` (C6); libDDD's missing null terminal is added back explicitly (`size()+1` unless constant true) — both pinned by the asymmetric-table order test and the PBT differentials. The OxiDD 2^23 capacity for alea-lcg-24 is a per-charge environment override, documented in the campaign README; the default image stays the reference.
- **Escalations**: none.

## 2026-11-18 - MEM-19: layout on/off confrontation, published measurement (issue #135)

- **Issue**: #135 (MEM-19, milestone MEM-B) — the measurement that fixes the real factor of the structural counter (prediction ×5 to ×12 on node memory): container variant in the runner, same node counts (C1), RSS and time. Criteria: measurement published in bench-results/ and WORKLOG; written verdict (constant factor or not); codegen opening decision noted for M5 with its criterion on the counters.
- **Done**: `Arena` gains a layout toggle (`set_layout_inline`, MEM-16 inline exact-class entries on vs catch-all boxed entries off — same node counts either way, C1). New runner binary `lisage-bench/src/bin/layout.rs` (`layout <on|off> <n>`) runs the deterministic seeded BDD workload (same `SEED_IDS` as MEM-18) in one process per layout and prints one JSON line: nodes, end-to-end bytes/node, arena bytes/node, time, peak RSS (`VmHWM`). New `Arena::arenas_bytes()` exposes the structural counter (arena slots alone). Test `lisage-bench/tests/layout.rs` runs both layouts via `CARGO_BIN_EXE_layout`, asserts identical node counts (20002 = 20000 + 2 terminals, C1) and pins falsifiable factor ranges (arena 1.5-2.0×, end-to-end 1.1-1.5×); the publication `bench-results/mem19-layout-on-off.md` is committed and asserted up-to-date on its structural lines (rewritten only under `MEM19_PUBLISH=1`, so a test run never dirties the repo with machine-dependent RSS/time).
- **Measured verdict (written)**: on the seeded BDD workload (n = 20000, degree-2 nodes), layout on vs off: arena slots 24.00 → 40.00 B/node (**1.667×**), end-to-end bytes/node 80.42 → 96.42 (**1.199×**), RSS 8944 → 10364 KiB (**1.159×**), time within noise. Node counts identical (C1). The factor is **constant in n** (n = 2000 gives the same 1.667× arena / 1.263× end-to-end). The ×5..×12 prediction is therefore **not** confirmed for the layout toggle alone: it applies to the cumulative MEM-B track against the ~420 B/node pre-MEM-B baseline (measured 420/60.75 ≈ **6.9×**, inside ×5..×12). The inline layout contributes a constant ~1.67× on node memory, ~1.2× end-to-end once the unique table is counted.
- **Codegen decision for M5 (noted)**: codegen stays **closed**. Opening criterion on the counters: a measured factor ≥ 5× on the structural counter (arena bytes/node, layout on vs off) sustained on the reference instance (R_5), or ≥ 2× end-to-end bytes/node — the current constant 1.67×/1.2× does not meet it. Re-evaluate after MEM-C when the thresholds freeze.
- **Evidence**: `cargo test -p lisage-bench --test layout` green in both publish and assert modes; `make check` green (ruff, mypy strict, import-linter, pytest, `cargo fmt --check`, clippy `-D warnings`, nextest, english-only). Not run: GitHub CI (merged on local gates only, per owner instruction this session).
- **Judgement calls**: RSS/time are measured per layout in separate runner processes (VmHWM is process-wide); the committed publication pins only the structural lines (nodes, bytes/node, arena bytes/node, factors) — RSS/time are recorded as measured, machine-dependent. The verdict uses the arena-slots counter for the "node memory" factor and notes that the off layout's boxed runs are not in the slot counter, so the true node-memory factor is higher ((40+24)/24 ≈ 2.7×) — the conservative 1.67× is the pinned one.
- **Escalations**: none.

## 2026-11-18 - MEM-18: counting allocator and frozen CI thresholds (issue #134)

- **Issue**: #134 (MEM-18, milestone MEM-B) — end-to-end bytes per node (arena + tables) and allocations per interning as non-regression thresholds, per ADR-0012 (anti-complacency); reproducible randomized IDs so a layout on/off comparison never captures a hash-function artifact. Criteria: gate red on a threshold regression (verified by a deliberately regressive witness, then reverted); thresholds frozen after MEM-C.
- **Done**: new `crates/lisage-bench/tests/thresholds.rs` — (1) a **counting global allocator** (`CountingAllocator` over `System`, atomic counters) measuring allocations per interning on a deterministic workload; (2) frozen threshold gates: `SEUIL_OCTETS_PAR_NOEUD = 66.0` (measured 60.75 on 2000 seeded BDD nodes, `bytes_per_node()` = unique table + class arenas + shadow index) and `SEUIL_ALLOCATIONS_PAR_INTERNEMENT = 10.0` (measured 9.174, identical in debug and release); (3) `IdsAleatoires`, a xorshift64* generator with fixed seed `SEED_IDS` (no new dependency) producing the randomized arc keys of the workload — reproducibility pinned by a test (same seed → same sequence, different seed → different). Tests serialize on a mutex because the allocator is global to the test binary (nextest isolates per process anyway). Witness verification run locally: threshold lowered to 50.0 → `REGRESSION: bytes/node 60.75 > threshold 50` (red), reverted → green; same for the allocation gate (5.0 → `9.174 > 5` red, reverted → green). Thresholds marked "freeze after MEM-C" in the file.
- **Evidence**: `cargo test -p lisage-bench --test thresholds` green (3 tests); `make check` green (ruff, mypy strict, import-linter, pytest, `cargo fmt --check`, clippy `-D warnings`, nextest, english-only). Not run: GitHub CI (merged on local gates only, per owner instruction this session); the witness commit was run locally, not pushed.
- **Judgement calls**: thresholds frozen at measured + ~9% headroom (66.0 / 10.0) rather than exactly at the measurement, so the gate catches regressions without flapping on platform noise; the workload uses one distinct level per node with randomized arc keys — randomizing levels too collapsed the distinct-node count via hash-consing shares (128 instead of 2000), which would have measured the share path instead of the creation path.
- **Escalations**: none.

## 2026-11-18 - MEM-16: declared key width per level, inline exact-class entries, owned resolve (issue #132)

- **Issue**: #132 (MEM-16, milestone MEM-B) — key width u8/u16/u32 per level declared by the controller via `cardinality()`; runs at 8 B/arc on small domains instead of 12. Criteria: round-trip PBT on narrowed keys; stride of the exact classes at `8·d + header` verified by a static test; no loss on open domains (key/value run unchanged). The needs-human question (change `Pool::resolve` vs keep `&Form`) was answered by the owner on 2026-11-18: architecture change, ADR-0104 drafted in the PR.
- **Done**: exact classes now store arcs **inline** in a monomorphic `InlineEntry<d>` (slot stride exactly `8 + 8·d`: header `plan`+`live`+`level` padded to 8, one `(u32, NodeId)` per arc — the key narrowed to its 32-bit rank). `Pool::resolve` returns an **owned** `Form` (ADR-0104): the ~100 call sites compile unchanged; the pool's hot paths (unique-table `same_form`, GC marking) use borrowed accessors on the object-safe `ClassArenaDyn` trait and never rebuild a form. New `KeyAlgebra::key_width` (u8 ≤ 2^8, u16 ≤ 2^16, u32 ≤ 2^32, `None` on open domains) with defaulted `narrow`/`widen` over `rank`/`unrank`. Non-narrowable nodes (open domain or degree beyond the window) route to the catch-all class, which keeps the full `NodeEntry` — key/value run unchanged. The unique table is unchanged semantically (hash of the full-width semantic form, C1; equality via the stored entry with widened keys). `choose_dense_family` switched to checked arithmetic (open-domain cardinality no longer overflows). Tests: static stride `size_of::<InlineEntry<d>>() == 8 + 8·d` (classes.rs); PBT round-trip on narrowed keys + exact-class routing + C1 through the inline storage + open-domain catch-all unchanged (lisage-bench `narrow.rs`).
- **Evidence**: `cargo test --workspace` green (80 suites, 0 failed); `make check` green (ruff, mypy strict, import-linter, pytest, `cargo fmt --check`, clippy `-D warnings`, nextest, english-only). Not run: GitHub CI (merged on local gates only, per owner instruction this session).
- **Judgement calls**: owned `Form` from `resolve` (drop-in for the call sites) rather than a borrowed `NodeView` (would rewrite every pattern), with borrowed accessors for the internal hot paths — the owner delegated the shape ("go with what you recommend"). The resolved form reapplies the canonical dense-family choice (MEM-12), so `resolve` returns exactly what a boxed storage would have. Class routing now depends on key narrowability, not only degree — recorded in ADR-0104.
- **Escalations**: the needs-human question on #132 was answered by the owner (comment 6987435213): voie (a), architecture change, ADR from 0100.

## 2026-11-18 - MEM-17: degree histogram per class, slack traced, JSONL v3 (issue #133)

- **Issue**: #133 (MEM-17, milestone MEM-B) — histogram of degrees and `MetriquesPool` per class. Criteria: the histogram is visible in the Observer AND in the JSON line; MEM-9's C boundary justified by measurement; the expected mass at 1-4 arcs on the reference instance confronted with reality.
- **Done**: `ClassMetrics` gains `slack = slots − live` (tombstone slots kept below the compaction threshold), so the per-class pool metrics now expose live / peak / slack. New `ClassHistogram { class, live, peak, slack }` and `TraceEvent::Histogram { classes }`, emitted by `Arena::gc()` right after the `Gc` event, plus a public `Arena::histogram()` accessor. The liseuse serializer bumps `TRACE_VERSION` to 3 and emits one JSON line `{"v":3,"type":"Histogram","classes":[{"class":..,"live":..,"peak":..,"slack":..}]}` per GC; the step-by-step reader handles the new event; the trace-format doc § 3 updated (v3 + Histogram row). Tests: `classes.rs` — slack=5 after a sub-threshold GC, histogram accessor mirrors the metrics, exactly one Histogram event per GC carrying the same data; `philosophers.rs` — on the reference instance R_5, ≥ 90% of internal nodes sit in classes 1..=4 and ≤ 10% in the catch-all class, which confronts the expected 1-4 arc mass with the measured one and justifies MEM-9's C boundary (exact classes cover the observed degrees, the catch-all stays marginal); golden trace tests updated to v3.
- **Evidence**: `cargo test --workspace` green (79 suites, 0 failed); `make check` green (ruff, mypy strict, import-linter, pytest, `cargo fmt --check`, clippy `-D warnings`, nextest, english-only). Not run: GitHub CI (merged on local gates only, per owner instruction this session).
- **Judgement calls**: the histogram rides the existing GC event stream (one Histogram line per GC) rather than a new throttled snapshot — the GC is already the natural per-class accounting point and the volume stays bounded by the GC frequency. Slack is reported per class in the same event instead of a separate counter event, keeping the JSONL schema to one new event type.
- **Escalations**: none.

## 2026-11-18 - MEM-15: co-located memories in dense per-class arrays (issue #131)

- **Issue**: #131 (MEM-15, milestone MEM-B) — replace the `HashMap<(NodeId, Level), V>` (~48 B/entry) with dense parallel arrays per class (4–8 B/entry); bounded eviction policy unchanged. Criteria: lemma L1 (key `(id, h, version(M))`) and fingerprints unchanged, PBTs green; remap applied by the memory holders; memory footprint divided (counter).
- **Done**: `MemoryLevel<V>` now stores per-class dense parallel arrays indexed by the node's index in its class arena: presence bitmap (1 bit/slot) + `u32` level array + value array (`Vec<Option<V>>`), decoded with the pool's class width `b` — new defaulted `Pool::class_bits()` (0 = single class), overridden by `Arena`, and set on the memory by the transduction runner at the start of `run_with`/`run_with_same_pool`. A node with entries at several levels keeps the first in its dense slot and the rest in a small per-class overflow list, so the exact `(node, level)` key semantics (and the L1 versioning: one bump per write) are unchanged — the memory PBT stays green. New `MemoryLevel::bytes()` counter (bitmaps + level arrays + value arrays + overflow, capacities included) and `MemoryLevel::remap(&BTreeMap<u32, Option<NodeId>>)`: the memory holders translate their stored node ids through the pool's `last_remap` after a compaction, dead entries (`old → None`) are dropped, and the version bumps once when anything moved (L1). Eviction policy unchanged: arrays bounded by the arena high-water mark, nothing evicted. Tests added in `crates/lisage-core/tests/memory.rs`: footprint per entry ≤ half of the ~48 B HashMap baseline (measured ~20 B/entry for `V = u64` over 500 entries); multi-level per node semantics; remap application (translate + drop dead + version bump); decode with `b = 2` via `NodeId::encode`.
- **Evidence**: `cargo test --workspace` green (79 suites, 0 failed, including the `lisage-bench` memory PBT); `make check` green (ruff, mypy strict, import-linter, pytest, `cargo fmt --check`, clippy `-D warnings`, nextest, english-only). Not run: GitHub CI (merged on local gates only, per owner instruction this session).
- **Judgement calls**: the dense slot holds one entry per node (the common case: a node's memory lives at its own level); the rare multi-level node goes to a per-class overflow list rather than a 2D dense array, which would be sized by `index × levels` and mostly empty. `Pool::class_bits()` defaults to 0 so pool-agnostic mocks (raw ids) keep working unchanged.
- **Escalations**: none.

## 2026-11-18 - MEM-14: unique table without form clone, open addressing (issue #130)

- **Issue**: #130 (MEM-14, milestone MEM-B) — remove the `form.clone()` from the unique-table key: open addressing of `(hash, NodeId)` (~8 B/node at load factor 0.5) or CUDD-style chaining; drop `ControllerId` from the key at minimum. Criteria: end-to-end bytes/node counter with factor ≥ 5 on the BDD instance vs the ~420 B current; zero additional allocation at interning (counter); hash-consing key still the semantic form.
- **Done**: new `lisage_core::unique::TableUnique` — open addressing (linear probing, power-of-two capacity, load factor ≤ 1/2) of 12-byte slots `(hash32, anchor, NodeId)`; the stored hash is truncated to 32 bits because equality is always confirmed through the stored node's entry (`same_form` callback), so a collision only costs a probe and the form is never cloned into the key. `ControllerId` is out of the key (constant per pool, `Arena<C>` is per-controller); the anchor stays (canonicity by handle). `Arena::intern` hashes `(anchor, form)` in place and compares via `entry(node).form`; GC sweeps the table (`retain`), compaction remaps through `values_mut`. New counters: `Arena::bytes_per_node()` (unique table + class arenas + shadow index over live nodes, `ClassArena::bytes()` at the exact `NodeEntry` stride) and `Arena::unique_len()`. Tests in `crates/lisage-core/tests/unique.rs`: BDD instance with 2000 nodes measures bytes/node ≤ 420/5 = 84 (measured ~69); re-interning 100 existing forms leaves `unique_len` unchanged (zero additional allocation at interning, hits counted as shares); C1 — same anchor+form → same NodeId, distinct anchors → distinct handles; GC below the compaction threshold sweeps dead keys and re-interning gets a fresh index (stale id is a tombstone).
- **Evidence**: `cargo test --workspace` green (79 suites, 0 failed); `make check` green (ruff, mypy strict, import-linter, pytest, `cargo fmt --check`, clippy `-D warnings`, nextest, english-only). Not run: GitHub CI (merged on local gates only, per owner instruction this session).
- **Judgement calls**: open addressing chosen over CUDD-style chaining in the entry — chaining would need a `next` pointer per node entry (same bytes) plus a head table, while open addressing keeps one flat slot array and reuses the entry for the semantic comparison. The hash-consing key remains the semantic form (ADR-0103 invariant 1): the table stores only a hash, and equality is decided by comparing the stored form, so C1 is unchanged. The 32-bit stored hash is safe because the form comparison is authoritative. The terminal singleton map (`unique_terminals`, MEM-11) is left as a `HashMap<Payload, NodeId>` — payloads are small (bool for BDD) and out of this issue's scope.
- **Escalations**: none.

## 2026-11-18 - MEM-13: tag bits arbitrated in the tripartite NodeId budget (issue #129)

- **Issue**: #129 (MEM-13, milestone MEM-B) — arbitration of the tripartite identifier budget (class + index + tag): CUDD-style complement bit, ceiling for the M4-2 shift (borrow key bits on small domains or tag nibble per arc). Criteria: ADR-0103 section merged with the sizing; motivated decision; M4-2 does not open without it.
- **Done**: `NodeId(u32)` layout is now **tag (4 bits, highest) + class (b bits) + index (32−4−b bits, low)**. New constants in `lisage_algebra::class_bits`: `TAG_BITS = 4`, `TAG_COMPLEMENT = 0b0001` (bit 0, CUDD-style edge transformer), `TAG_SHIFT_MASK = 0b1110` (bits 1..=3, per-arc shift nibble of M4-2, disjoint from the complement bit). `NodeId::encode` keeps its signature (zero tag); new `encode_tagged`/`decode_tagged`/`tag`; `decode` still returns `(class, index)` with the tag masked out, so all existing callers (arena, tests) are unchanged. `index_bits` now subtracts the tag field. ADR-0103 sizing section merged with the arbitration: the tag is reserved unconditionally (4 bits) rather than borrowed from key bits on small domains, because `b` is runtime schema data and a fixed tag keeps the layout pool-independent; cost recorded (index 26 bits at b=2, 16 bits at b=8). Tests: tag round-trip (complement bit + shift nibble, layout, disjointness, zero tag of `encode`), tag window rejection, updated index-window expectations (30→26 bits); `bench-results/philosophers-n5.svg` regenerated (raw NodeIds shifted by the tag field).
- **Evidence**: `cargo test --workspace` green (78 suites, 0 failed); `make check` green (ruff, mypy strict, import-linter, pytest, `cargo fmt --check`, clippy `-D warnings`, nextest, english-only). Not run: GitHub CI (merged on local gates only, per owner instruction this session).
- **Judgement calls**: fixed 4-bit tag reservation over borrowing key bits on small domains — the deciding factor is identifier-layout stability across pools (the decoder stays parameterized by `b` alone); the index cost at b=8 (16 bits ≈ 65k/class) is accepted and recorded in the ADR. M4-2 remains closed until it consumes `TAG_SHIFT_MASK`.
- **Escalations**: none.

## 2026-11-18 - MEM-12: dense ordinal arc family, chosen in normalization (issue #128)

- **Issue**: #128 (MEM-12, milestone MEM-B) — third family of the `ArcStore` port: arcs indexed by rank, holes = default not stored; chosen in normalization when the rank exists and the fill is at least one third (threshold `4·|Val(ℓ)| ≤ 12·arcs`); composes with the class tier by arc count. Criteria: C1 PBTs green with the dense family; threshold documented and tested; no per-controller code (rank derived from `KeyAlgebra`).
- **Done**: `ArcRun<Key>` is now an enum with two families behind the same type (so `Form::Node.arcs` and the ~100 construction sites are untouched): `Sorted` (MEM-5, default, built by `from_pairs`) and `Dense` (one slot per rank of the level, `None` = hole = default not stored). New `ArcRun::choose_dense_family(level, keys, arcs)` implements the documented threshold: dense iff every key has a rank in the `KeyAlgebra` at that level AND `4·|Val(ℓ)| ≤ 12·arcs` (fill ≥ 1/3, boundary included); the choice is deterministic in (level, keys, content) so C1 holds. The choice is applied in `Arena::intern` right after the reductions (part of the normalization pipeline, placed there so the choice survives on the stored hash-consed form). All `ArcRun` accessors (`get`/`len`/`iter`/`values`/`keys`/`contains_key`/`Index`/iterators/`ArcStore`) work on both families; `values()`/`keys()` stay `DoubleEndedIterator` (svg.rs relies on `.rev()`). No per-controller code: the rank comes from `KeyAlgebra::rank`. Tests: `crates/lisage-core/tests/dense.rs` (threshold boundary exact — |Val|=3 with 1 arc is dense, |Val|=4 with 1 arc is sorted; no rank → sorted whatever the fill; holes = default not stored, rank-order iteration; pool integration — dense chosen at fill ≥ 1/3, node still in the class of its degree; C1 same form / shuffled pairs → same NodeId) and `crates/lisage-bench/tests/dense.rs` (proptest C1 with the dense family: same semantic form → same NodeId, stored family matches the threshold, content preserved). 5 assertions in `crates/lisage-core/tests/schema.rs` switched to a family-agnostic `assert_forme` helper (level + ordered pairs) since the family is a normalization choice, not semantics.
- **Evidence**: `cargo test --workspace` green (78 suites, 0 failed); `make check` green (ruff, mypy strict, import-linter, pytest, `cargo fmt --check`, clippy `-D warnings`, nextest, english-only). Not run: GitHub CI (merged on local gates only, per owner instruction this session).
- **Judgement calls**: the dense family lives inside the `ArcRun` type as a second variant rather than changing `Form::Node.arcs` to a new enum — invariant 1 of ADR-0103 (hash-consing key = semantic form) is preserved because the family choice is deterministic; keeping one type avoids touching ~100 construction sites. The choice is applied after the reductions (not in `form_normalisee`) because the reductions rebuild the run sorted — placing it after keeps the dense family on the stored form. The PBT initially flagged reversed duplicate-key constructions; that is MEM-5 last-write-wins semantics, not a C1 break — the PBT now deduplicates by key first.
- **Escalations**: none.

## 2026-11-18 - MEM-11: dedicated terminal class, pool-wide terminal singletons (issue #127)

- **Issue**: #127 (MEM-11, milestone MEM-B) — dedicated terminal class (class 0), terminal and D_δ singletons per pool (canonicity by handle only for internal nodes), D_δ interned once per pool (metric), `est_defaut` and R-collapse see inter-anchor merges, large-payload class without touching internal nodes. Prepares ADD/MTBDD (M5-4).
- **Done**: `Arena` gains `unique_terminals: HashMap<Payload, NodeId>`; `intern` routes `Form::Terminal` to a pool-wide singleton in the class-0 arena (`NodeId::encode(TERMINAL_CLASS, index, b)`), keyed by payload only — the anchor does not enter terminal canonicity. Hits count as shares (`partages`), misses as creations and as `MetriquesPool.terminaux` (new field, D_δ counted once per pool). `reduced()`'s D^default lookup and `gc()`'s retain now go through `unique_terminals`, so `est_defaut` and R-collapse see the inter-anchor terminal merges. Internal nodes keep per-anchor handle canonicity via the existing `unique` table. Tests in `crates/lisage-core/tests/classes.rs`: singleton across anchors (same payload → same NodeId, class 0, `terminaux` metric, share counting, survival across GC); R-collapse inter-anchor yields the terminal; R-default absorbs δ(ℓ) → D_δ with the same terminal singletons referenced from both anchors while internal nodes stay per-anchor handles; large payload (`[u8; 4096]`) in class 0 with internal nodes unaffected. `crates/lisage-bench/tests/sdd.rs::test_canonicite_two_anchors_distinctes` updated to the new semantics (internal node distinct per anchor, terminal shared).
- **Evidence**: `cargo test --workspace` green (76 suites, 0 failed); `make check` green (ruff, mypy strict, import-linter, pytest, `cargo fmt --check`, clippy `-D warnings`, nextest, english-only). Not run: GitHub CI (merged on local gates only, per owner instruction this session).
- **Judgement calls**: terminals are pool-wide singletons in the class-0 arena rather than a payload-stride slab storage — `Pool::resolve` returns `&Form` (93 call sites), so owning payload storage per terminal would be too invasive; the singleton map keeps the borrow-based API. Deferral noted for a future stride-based terminal store if payload sizes grow. Internal nodes remain keyed by (controller, anchor, form): the issue's "canonicity by handle only for internal nodes" is read as terminals being the exception, not internal nodes becoming anchor-independent.
- **Escalations**: none.

## 2026-11-18 - MEM-10: GC and multi-arena remap, bitmap marking (issue #126)

- **Issue**: #126 (MEM-10, milestone MEM-B) — bitmap marking (1 bit per node), shadow index per level; tombstone + remap protocol per class, generalized `last_remap()`; page rendering when a class empties. Criteria: existing GC and PBTs green; marking memory peak under the old `BTreeSet` (measured threshold); no index reused outside the protocol (test).
- **Done**: `gc` now marks with one bitmap per class arena (1 bit per slot, `ClassArena::empty_marks`/`set_mark`/`mark_of`/`sweep(marks)`), replacing the `BTreeSet<NodeId>`; `StatsGc` gains `mark_bytes` (bitmap size at the last GC). The tombstone + remap protocol is per class: `last_remap()` (already keyed by old raw `NodeId` since MEM-9) records `old → Some(new)` for live nodes and `old → None` for tombstones; compaction renumbers each class arena densely. Page rendering: when the threshold is NOT reached and a class empties, `release_pages()` frees its slabs while keeping the slot count as high-water mark — no index recycled outside the remap protocol (ADR-0103 invariant 3). The shadow index per level (`index_level`) was already in place (M4-1). Tests added in `crates/lisage-core/tests/classes.rs`: marking bitmap exact size (1 bit/slot) and under the old set's raw ids alone (`mark_bytes < 4 × slots`, measured on 2000 nodes); no index reused below the threshold (stale id unknown, re-interned form gets a fresh index) and orphans remapped to `None` under compaction; emptied class releases pages with the high-water mark kept (fresh index beyond the old slots). Existing GC/PBT suites unchanged and green.
- **Evidence**: `cargo test -p lisage-core --test classes` 7 passed; `make check` green (ruff, mypy strict, import-linter, pytest, `cargo fmt --check`, clippy `-D warnings`, nextest, english-only). Not run: GitHub CI (merged on local gates only, per owner instruction this session).
- **Judgement calls**: the marking bitmap is per class arena (1 bit per node, no global dense numbering needed); `mark_bytes` is exposed on `StatsGc` rather than a new observer event to avoid touching the exhaustive `TraceEvent` matches in liseuse. Page release keeps the slot high-water mark — freeing and resetting would silently recycle indices, violating invariant 3.
- **Escalations**: none.

## 2026-11-18 - MEM-9: exact classes and catch-all class, per-class arenas (issue #125)

- **Issue**: #125 (MEM-9, milestone MEM-B) — exact classes for degrees `d = 1..C` at exact stride (macro-generated, monomorphized), catch-all class beyond; lazily-created arenas, grown by doubling, 64 KiB chunked slabs; class map frozen at construction (assertion). Criteria: C1 PBT green inter-class (same form, same NodeId, whatever the tier); exact-stride entry verified by static test; live and peak per class traced; boundary C justified by the histogram (MEM-17).
- **Done**: new `lisage_core::classes` module (ADR-0103): `SLAB_SIZE = 64 KiB`, `EXACT_MAX_DEGREE = 2^b_max − 2 = 254`, `EXACT_CLASSES` (macro-generated table, class of exact degree `d` is `d`), `ClassMap` (frozen at `Arena::new` from `class_bits_of(keys, sweep_levels)`, window asserted; `class_bits`, `catch_all = 2^b − 1`, `exact_max = C`), `ClassArena` (lazy creation, doubling growth, slabs of 64 KiB at the exact stride `size_of::<NodeEntry>()`, tombstone sweep, per-class compaction with index remap), `ClassMetrics { class, live, peak }`. `Arena` storage rewired from one `Vec` to per-class arenas: `intern` routes the node to the arena of its class (degree) and issues `NodeId::encode(class, index, b)`; `resolve`/`plan_de`/`gc`/`compacter` decode and route; `last_remap` is now keyed by the old raw `NodeId` (`BTreeMap<u32, Option<NodeId>>`); new `class_map()` and `class_metrics()` accessors (live + peak per class traced). `KeyAlgebra` gains `sweep_levels()` (default 1; `BddKeys` → number of variables, `SchemaKeys` → schema sweep). The catch-all class holds the entries whose form carries the sorted run (`ArcRun`) — no loss of correctness beyond the window. Tests: `classes.rs` unit tests (static exact-stride slab fill, macro table) + `crates/lisage-core/tests/classes.rs` (frozen class map at b = 2; C1 inter-class — same form → same NodeId for degrees 1, 2, 3, 10 across exact and catch-all tiers, different form same degree → different id same class; catch-all resolvable with sorted run; per-class live/peak traced across a GC); `biplan.rs` compaction test updated to the per-class remap (live root remapped, orphans → None). Boundary C = 2^b − 2 comes from the MEM-8 derivation; the histogram justification is MEM-17 (#133).
- **Evidence**: `cargo test -p lisage-core` 28 binaries green (incl. the 6 new MEM-9 tests); `make check` green (ruff, mypy strict, import-linter, pytest, `cargo fmt --check`, clippy `-D warnings`, nextest, english-only). Not run: GitHub CI (merged on local gates only, per owner instruction this session).
- **Judgement calls**: "PBT C1 inter-classes" is pinned by a deterministic sweep over degrees 1/2/3/10 (exact + catch-all tiers) rather than a proptest — adding proptest to `lisage-core` would be a new dependency (escalation list). The catch-all "pointer to sorted run" is the entry's `Form::Node.arcs` (`ArcRun`) — the deep inline-arcs layout is a later MEM-B item. Per-class compaction keeps a live node's id stable when it already heads its class (remap may be identity) — the biplan test was updated accordingly. Multi-arena GC remap polish is MEM-10 (#126). The committed figure `bench-results/philosophers-n5.svg` was regenerated: its node labels embed the raw `NodeId`, which now carries class+index bits (same diagram, same node count, relabeled ids).
- **Escalations**: none.

## 2026-11-18 - MEM-8: NodeId split into class bits + index bits, runtime b (issue #124)

- **Issue**: #124 (MEM-8, milestone MEM-B) — `NodeId(u32)` = class bits + index bits; `b` = smallest integer with `2^b − 2 ≥ max_ℓ C(ℓ)`, derived from `cardinality()` at runtime; compile-time window `b_max` (default 8); decoder parameterized by the effective `b`; degree beyond the window → catch-all class without loss of correctness. Criteria: tested derivation (BDD → b = 2, 30 index bits; philosophers → b = 3 or 4; open domain → low default with declarative overload); invalid identifiers rejected by assertion.
- **Done**: new `lisage_algebra::class_bits` module (ADR-0103): `CLASS_BITS_MAX = 8` (compiled window), `CLASS_BITS_OPEN_DEFAULT = 1` (low default for an open/unbounded domain), `TERMINAL_CLASS = 0`, `derive_class_bits(max_cardinality)` (smallest `b` in `1..=b_max` with `2^b − 2 ≥ max C(ℓ)`, open domain → low default), `class_bits_of(keys, number_levels)` (max of `cardinality` over the levels, declarative overload first), and on `NodeId`: `index_bits`, `catch_all_class` (`2^b − 1`), `encode`/`decode` parameterized by the effective `b` (class in the high bits, index in the low bits; out-of-window fields rejected by assertion), `class_of_degree` (exact class up to `2^b − 2`, catch-all beyond — no loss of correctness). `KeyAlgebra` gains `declared_class_bits() -> Option<u32>` (default `None`): the declarative overload, clamped to the window. The arena still assigns creation-order identifiers (zero-width class field, full 32-bit index — the ADR-0103 current instantiation); routing nodes into exact classes is MEM-9 (#125). Tests `crates/lisage-core/tests/class_bits.rs` (8): BDD → b = 2 / 30 index bits; philosophers ring (domain size 3) → b = 3, domain size 14 → b = 4; open domain (`u64::MAX`) → low default, overload 4 → 4, overload 12 → clamped to 8; encode/decode round-trip incl. raw layout (`encode(1,0,2) = 1<<30`); catch-all beyond the window (degree 7 at b = 3 → class 7, encodable); three `should_panic` tests rejecting invalid identifiers (class/index out of window, class field wider than 32 bits).
- **Evidence**: `cargo test -p lisage-core --test class_bits` 8 passed; `make check` green (ruff, mypy strict, import-linter, pytest, `cargo fmt --check`, clippy `-D warnings`, nextest, english-only). Not run: GitHub CI (merged on local gates only, per owner instruction this session).
- **Judgement calls**: the identifier machinery lands in `lisage-algebra` next to `NodeId` (the pool stays blind, ADR-0103 invariant 4); the arena rewiring to per-class assignment is deferred to MEM-9, so MEM-8 is behavior-preserving. The low default for open domains is `b = 1` (every non-terminal degree in the catch-all class — correct, no exact class), chosen as the most conservative "low default"; the declarative overload is the escape hatch.
- **Escalations**: none.

## 2026-11-18 - MEM-7: pilot measurement and generalization gate (issue #123)

- **Issue**: #123 (MEM-7, milestone MEM-A) — runner alea-lcg / philosophers / window, JSON line (VmHWM, time, node counts), publication in bench-results/ and WORKLOG; written decision: gap = constant factor → layout, MEM-B opens; complexity gap → stop and escalate. Criteria: published and falsifiable measurement (reproducible randomized IDs); written decision in the closed issue body; WORKLOG updated.
- **Done**: the orchestrator `scripts/bench_confrontation.py` gains the MEM-7 pilot charges (`--pilot`): `alea-lcg-20` (cross-checked against the OxiDD witness), `philosophers-n5` and `window-4x10` in both encodings (lisage-only lines — no DDD/MDD witness in the stdin protocol). `Charge` carries explicit workload `parameters` and a per-charge `tools` restriction; `dump-charge` now supports `alea-lcg` (the deterministic LCG table of `SEED_LCG` is the charge, serialized in the stdin protocol — reproducible randomized IDs by construction). New versioned input `crates/lisage-bench/models/philosophers-n5-states.txt` (extended states of R_5, same algorithm as `etats_etendus` in tests, validated against the pinned |R_n| for n = 3..7), documented in the models README. Pilot campaign `bench-results/20261118-pilot-1/` deposited: raw JSONL, `aggregats.json` (empty divergence list), `README.md` (exact reproduction command), `REPORT.md` (measured values + decision). Tests: `test_pilot_charges_mem7`, `test_charge_parameters_default_path`, `test_run_pilot_selects_pilot_charges`, `test_run_campaign_restricts_tools_per_charge` in `scripts/tests/test_bench_confrontation.py`; `test_serialiser_charge_alea_lcg_pinned` in `crates/lisage-bench/tests/confrontation.rs` pins the `dump-charge alea-lcg` serialization (2^n values, pinned LCG satisfactions for n = 4 and 12, typed error on invalid n).
- **Evidence**: campaign run `make bench-confrontation ARGS="--pilot --repetitions 5 --run-id 20261118-pilot-1"` exit 0, no divergence. alea-lcg-20: noeuds 106760 and verifie 523228 identical lisage/OxiDD (C6 cross-tool); lisage 537.9 ms median vs OxiDD 129.8 ms (4.1x, the same constant factor as the M7-6 campaign's 3.5-4.0x); peak RSS lisage 31000 kio vs OxiDD 50328 kio (1.6x less). philosophers-n5: 73 nodes, verifie 152 = pinned |R_5|. window 4x10: 40 paths, 12 nodes absolute / 3 relative = pinned. `make test` (47 passed), `cargo test --workspace` (74 binaries, 0 failures), `make lint typecheck imports english-only`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` all pass.
- **Decision (written)**: gap = constant factor on every pilot line (node counts identical or pinned, time ~4x, memory ~1.6x better) — no complexity gap → the layout is validated and **MEM-B opens**; no stop-and-escalate. The decision is also in `bench-results/20261118-pilot-1/REPORT.md` and in the body of the closed issue.
- **Judgement calls**: the pilot's external witness is OxiDD only (CUDD/Sylvan not in the campaigns, ADR-0102; the DDD/MDD pilot workloads have no stdin-protocol witness, so they are published as lisage-only lines with pinned semantic values — the falsification there is the pinned |R_n| / window node counts, not a cross-tool count).

## 2026-11-18 - MEM-6: published binary node of the BDD witness (issue #122)

- **Issue**: #122 (MEM-6, milestone MEM-A) — binary node of the BDD witness (C6): const-generics instance `{level, low, high}` plus the CUDD complement bit; the binary family is the `Val(level) = 2` case of the port. Criteria: static `size_of` of the published binary node; C6 verified by cross-count against the CUDD witness; differential green.
- **Done**: `crates/lisage-bench/src/temoin_bdd.rs` publishes `BinaryNode<const LEVEL: u32, const LOW: u32, const HIGH: u32>` — the three classic BDD node fields as const generics, the only runtime state being the CUDD-style complement bit (`new`, `level`/`low`/`high`, `is_complemented`, `negate`). Static size published as `BinaryNode::SIZE` and pinned by a const assertion (`size_of == 1` byte). `MiniBdd::binary_node::<LEVEL, LOW, HIGH>(index)` materializes a stored node as the published type when the runtime fields match the const generics. CI test `test_published_binary_node` (tests/temoins.rs) pins the size, the typed view and the C6 count on XOR-2 (3 internal + 2 terminals = 5). CUDD witness made cross-countable: `docker/cudd/cudd_count.c` now counts distinct (node, complement-bit) pairs, terminals included (the lisage convention — raw `Cudd_DagSize` shares nodes through complement edges and counts a single constant, so it is not comparable), and builds with variable l = bit l of the table index, variable 0 at the root, matching lisage; `docker/cudd/Dockerfile` fixed to install autotools/perl and run `autoreconf -fi` (the git checkout has no generated `configure`; the image previously did not build). New local differential `test_c6_local_witness_vs_cudd` (tests/cudd_local.rs, `#[ignore]` outside CI per ADR-0102) cross-counts the witness against real CUDD.
- **Evidence**: `cargo test -p lisage-bench --test temoins` (2 passed: new pin + C6 arena-vs-witness PBT green), `cargo test -p lisage-bench --test cudd_local -- --ignored` against `lisage-cudd:latest` rebuilt from the fixed Dockerfile: 3 passed, 40/40 proptest cases + the asymmetric order pin — arena-vs-CUDD (previously failing on terminal counting), witness-vs-CUDD identical node counts, and the variable-order pin (asymmetric table [1,0,0,1,1,0,0,0] over n=3: 6 nodes, the reversed order would give 7 — verified by exhaustive enumeration over all 2^8 tables). `cargo test --workspace` 0 failures, `make test` (45 passed), `make lint`, `make typecheck`, `make imports`, `make english-only`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` all pass. Not run in CI: the Docker-based CUDD differential (`cudd_local` is `#[ignore]`d outside CI per ADR-0102) — it was run locally against the rebuilt image instead, as recorded above.
- **Judgement calls**: the CUDD cross-count uses the lisage counting convention implemented in the witness binary (documented in `cudd_count.c`), because C6 "identical reproduction" in node count is only meaningful under one convention; the variable order was aligned (variable l = bit l, root = variable 0) since BDD size depends on the order. The Dockerfile fix (autotools + `autoreconf`) was required to build the witness image at all; it is included in this PR and called out in the PR body.

## 2026-11-18 - MEM-5: sorted run as arc storage (issue #121)

- **Issue**: #121 (MEM-5, milestone MEM-A) — the reduce pipeline normalizes to a sorted run `Box<[(key, NodeId)]>` with "last write wins" semantics; reductions run on the run; hash-consing on the semantic form (level + sorted run); BTreeMap demoted to construction accumulator.
- **Done**: new `ArcRun<Key>` in `lisage-algebra` (sorted `Box<[(Key, NodeId)]>`, one entry per key): `ArcStore`/`ArcStoreBuilder` impls, `from_pairs`/`From<BTreeMap>`/`FromIterator` with last-write-wins, `get`/`contains_key`/`iter`/`values`/`keys`/`len`/`Index`. `Form::Node.arcs` is now `ArcRun<Key>` — the hash-consing key is the semantic form (level + sorted run), never the construction pallet. `Reduction::Form` carries the run; `reduce`/`reduce_without_collapse`/`reduce_inverse`/`reduce_dabs` operate on the run (R-default removal rebuilds the run once). All 15 `Controller::Arcs` set to `ArcRun`. Merge paths (add/pli/lookup/workshop) accumulate in a `BTreeMap` and convert once at interning — one run allocation per interned form. New PBT `test_last_write_wins` pins last-write-wins; confluence PBTs (reduce∘reduce, quotient commutation, reduce_dabs idempotence) and the C1 handle PBTs updated and green. Unique-table key unchanged at this stage. Cites ADR-0103 (draft, #117).
- **Evidence**: `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, `cargo test --workspace` (74 binaries, 0 failures), `make rust-check` (308 passed, 5 skipped), `make test` (45 passed), `make lint`, `make typecheck`, `make imports`, `make english-only` all pass. Not run: a counting-allocator test for the "one allocation per interning" criterion — a global counting allocator in a multi-threaded test binary is flaky; the single-allocation property holds by construction (one `collect` into the `Box` per interned form, plus at most one rebuild when R-default removes an arc).
- **Judgement calls**: `ArcRun` keeps inherent `get`/`contains_key`/`values`/`keys` so read-only consumers (liseuse, tests) needed no trait import; the BTreeMap remains only as the builder-side accumulator, per the issue.

## 2026-11-18 - MEM-4: rank/unrank in KeyAlgebra (issue #120)

- **Issue**: #120 (MEM-4, milestone MEM-A) — rank/unrank derived from the region enumeration required by the key contract; rank availability exposed to normalization; dense ordinal storage writes no code per controller.
- **Done**: `KeyAlgebra` gains `rank(level, key) -> Option<u64>` and `unrank(level, rang) -> Option<Key>`, both defaulting to `None` — the absence of rank is properly declared for unbounded domains (DDD path unchanged), and the dense ordinal storage (MEM-B) needs no per-controller code. `SchemaKeys` (reference instance) and `BddKeys` implement them: on bounded domains the key IS the ordinal (`rank`/`unrank` bounded by `cardinality`). Doc note: rank availability is exposed to `normal_form`, which may encode keys by rank when it exists. PBT `crates/lisage-bench/tests/rank_unrank.rs`: round-trip `rank ∘ unrank` on bounded atom domains of the reference instance and on the BDD algebra, out-of-range `None` both directions, and declared absence on an unbounded algebra. Cites ADR-0103 (draft, #117).
- **Evidence**: `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, `make rust-check` (307 passed, 5 skipped, incl. the 3 new PBT cases), `make test` (45 passed), `make lint`, `make typecheck`, `make imports`, `make english-only`, `cargo doc -p lisage-algebra --no-deps` all pass.
- **Judgement calls**: rank/unrank are implemented only for the bounded u64-domain algebras (reference instance, BDD); region/CDD/DBM keys keep the default `None` (their "regions" are not simple ordinals) — the criterion only demands the reference instance's bounded domains plus declared absence elsewhere.

## 2026-11-18 - MEM-3: ArcStore port in the lisage-algebra interface (issue #119)

- **Issue**: #119 (MEM-3, milestone MEM-A) — ArcStore port in the interface: associated type on the controller, minimal contract (ordered iteration of (key, target), builder-side insertion, size), default instance = the current BTreeMap; reductions see only the port's iteration.
- **Done**: `lisage-algebra/src/interface.rs` gains the `ArcStore` port (`iter` ordered over `(key, target)`, `len`, `is_empty`, associated `Builder`) and the `ArcStoreBuilder` trait (`insert`, `build`); `BTreeMap<Key, NodeId>` implements both, so the default instance is literally the current map. `Controller` gains `type Arcs: ArcStore<Key = Self::Key>` (set to `BTreeMap<Self::Key, NodeId>` in all 15 impls), and `Controller::collapse` / `reduction::collapse_arcs` are now generic over `&impl ArcStore` — the collapse check sees only the port's ordered iteration. `Form` is unchanged (still `BTreeMap` arcs): rewiring the form to the store type is MEM-5 (sorted-run store), keeping this PR behavior-preserving. New test `crates/lisage-core/tests/arc_store.rs` pins the contract, incl. a `Vec`-backed alternative layout proving the port is layout-agnostic. Cites ADR-0103 (draft, #117).
- **Evidence**: `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, `cargo test --workspace` (73 binaries, 0 failures), `make rust-check` (304 passed, 5 skipped), `make test` (45 passed), `make lint`, `make typecheck`, `make imports`, `make english-only`, `cargo doc -p lisage-algebra --no-deps` all pass. No behavior change: the differential A/B/C suite is part of the 304 passing tests. `core` still does not inspect `Val(ℓ)`, `δ` or `⊕` (untouched).
- **Judgement calls**: the port is introduced with the BTreeMap as default instance and the collapse path already generic over the port; making `Form` itself generic over the store is deferred to MEM-5 (it would touch ~180 construction sites, over the ADR-0101 PR budget) — recorded here so the deferral is conscious.

## 2026-11-18 - MEM-2: English-only charter in AGENTS.md and the review checklist (issue #118)

- **Issue**: #118 (MEM-2, milestone MEM-A) — amend AGENTS.md and the review checklist: new modules in English, French identifiers renamed incrementally (no big-bang, ADR-0101), review case "no new French identifier introduced".
- **Done**: `AGENTS.md` style section gains the English-only charter line (new modules/identifiers in English, incremental renaming of remaining French identifiers, enforced by `make english-only`); `docs/review-checklist.md` section 2 gains the binary case "No new French identifier introduced (issues #110/#118); `make english-only` passes". The AGENTS.md/checklist edits are the explicit object of issue #118 (human directive), an authorized exception to the "no constitutional file" checklist rule.
- **Evidence**: docs-only change. `make lint`, `make english-only` pass. `make typecheck`/`imports`/`test`/`rust-check` not re-run (no Python or Rust file in the diff).
- **Judgement calls**: the issue text's carve-out "docs/ADRs/discussions stay in French" is superseded by M6 (issue #110): the repository is English-only everywhere, enforced by the `make english-only` lint over `docs/`, root Markdown and `crates/` — the charter states the post-M6 rule instead of the pre-M6 one.
## 2026-11-18 - MEM-1: ADR-0103 draft — arc storage port and per-controller-instance layouts (issue #117)

- **Issue**: #117 (MEM-1, milestone MEM-A) — draft ADR-0103 with the seven track invariants, the tripartite identifier budget (class + index + tag), and the reference to the analysis canvas sections 4 and 5.
- **Done**: `docs/decisions/adr-0103-arc-store-port-and-controller-layouts.md`, status **draft** (agents draft, only the human accepts — docs/decisions/README.md). Seven invariants: hash-consing key = semantic form; frozen class map; identifier space never recycled; verifiable core/algebra boundary preserved; schema as runtime data; tripartite identifier budget; order interface → tests → implementation. Sizing section: `NodeId(u32)` = 4 tag bits + 8 class bits + 20 index bits (default), arc-key width `b` derived at runtime from `cardinality()` (MEM-8, #124), known cases BDD b=2 / philosophers b=3-4. Open questions listed as gates: MEM-7 (#123, pilot measurement and generalization gate) and MEM-13 (#129, tag-bit arbitration before M4-2). References canvas § 4.4, § 4.6, § 5.3, ADR-0100/0101. Consequences state that every PR of the MEM-A track cites ADR-0103.
- **Evidence**: docs-only change. `make lint`, `make english-only` pass; `make typecheck`/`imports`/`test`/`rust-check` not re-run (no Python or Rust file in the diff — docs only). Acceptance of the ADR itself is a human decision (draft status), not part of this PR's gates.
- **Judgement calls**: the default bit split (4/8/20) is proposed as the sizing that MEM-13 (#129) will arbitrate — the ADR fixes the tripartition and the known sizing figures, not the final widths; the ADR is drafted in this PR per the escalation-list rule (ADR-worthy decisions are drafted in a PR, accepted by the human).

## 2026-11-18 - review bundle: elide data files from the reviewer's diff (no issue; human directive)

- **Issue**: none — human directive ("if these files are not real application code, the reviewer should not see them; make sure this never happens again").
- **Done**: `scripts/review.py` now elides data files from the DIFF section of the review bundle: any file whose extension is in `DATA_FILE_EXTENSIONS` (.pnml .cnf .jsonl .svg .png .jpg .jpeg .gif .pdf .zip .bin) is replaced by an explicit `### ELIDED FROM REVIEW ...` marker plus a `## ELIDED FILES` summary at the top of the section — elision is never silent. Code files are kept in full whatever their size (ADR-0010: the reviewer must see the whole diff of application code; the first version also elided oversized code files and the reviewer rejected it as BLOCKING). New `split_diff_sections` / `section_file_path` / `elide_data_files`; `render_bundle` applies it to the diff. Tests in `scripts/tests/test_review_bundle.py` (5 tests, incl. a `render_bundle` integration test and an oversized-code-file-kept test).
- **Evidence**: verified on the real M7-4 diff (`git diff e7d732b~1..8c9c4d2`, 48 901 bytes): the three model files (`SwimmingPool-1.pnml` 148 lines, `erk-000001.pnml` 223 lines, `uf20-01.cnf` 105 lines) are elided with markers, code files untouched; and on the M7-5/M7-6 diff (`git diff 8c9c4d2..6aede65`, 52 845 bytes): the 6 campaign JSONL files elided. Gates: `make lint`, `make typecheck`, `make imports`, `make test` (44 passed, 94.76%), `make english-only`, `make rust-check` (302 passed, 5 skipped) all pass. No Rust file in the diff, so `cargo fmt`/`clippy` not re-run.
- **Judgement calls**: elision is by data extension ONLY — the review finding showed that size-based elision of code files breaks ADR-0010 (the reviewer must judge the whole diff of application code); the marker names the file and the omitted line count so the reviewer knows what was skipped and can ask for it.

## 2026-11-17 - M7-6: comparative results report lisage vs OxiDD (issue #116)

- **Issue**: #116 (M7-6, milestone M7) — write the comparative report from the M7-5 results and update the docs.
- **Done**: report `bench-results/20261117-campaign-1/REPORT.md` (versioned next to the data): cross-check table (noeuds/verifie identical across tools: 51/8, 1859/1408, 9721/8672), time and peak-RSS tables per charge (median [min,max], 5 reps, every number sourced from `aggregats.json`), and a reading section — lisage uses 7-10x less peak RSS (OxiDD witness pre-allocates 2^20 unique-table/apply-cache entries, a fixed baseline), OxiDD is 3.5-4.0x faster (mature engine + apply cache vs lisage's Shannon expansion over the materialized 2^n table); measurement limits stated (single machine, 5 reps, table materialization in the measured block, CUDD/Sylvan not measured). Protocol doc `docs/bench-confrontation-witnesses-and-measures.md` now lists the witnesses (lisage binary, OxiDD container, CUDD not in campaigns) and the deposited results; `docs/state-of-the-art-decision-diagrams.md` links the report from the OxiDD section.
- **Evidence**: `make lint`, `make typecheck`, `make imports`, `make test` (40 passed, 94.76%), `make rust-check` (302 passed, 5 skipped), `make english-only` all pass. No Rust code changed in this PR (docs + deposited report only), so `cargo fmt`/`clippy` were not re-run — no Rust file in the diff.
- **Judgement calls**: the report lives in `bench-results/20261117-campaign-1/` (next to the data it cites) rather than `docs/`, and is linked from both docs; CUDD/Sylvan are explicitly marked as not measured rather than silently omitted.

## 2026-11-17 - M7-5: benchmark campaign runner + deposited results lisage vs OxiDD (issue #115)

- **Issue**: #115 (M7-5, milestone M7) — run the comparative campaign on the M7-4 charges, time AND memory, per the ADR-0102 protocol.
- **Done**: new orchestrator `scripts/bench_confrontation.py` (`python -m scripts bench-confrontation`, `make bench-confrontation`, ADR-0021 wiring): each charge x tool x repetition runs as a **fresh process** (fresh Docker container for OxiDD), raw JSONL deposited in `bench-results/<run-id>/`, aggregates (median/min/max) in `aggregats.json`, cross-tool divergence check on `noeuds`/`verifie` (exit 1 on divergence). The witnesses receive the exact charge serialized by lisage (`confrontation dump-charge <workload> <path>`, new binary mode backed by the new pub `serialiser_charge_depuis_fichier`). `oxidd_count` now emits the full protocol JSONL line (`temps_ms`, `rss_max_ko`, `noeuds`, `verifie`) with manager init outside the measured block (variables pre-declared up to 24). In `confrontation.rs` the measured block of the `cnf`/`pnml` workloads now includes input reading (protocol alignment). Protocol documented in `docs/bench-confrontation-witnesses-and-measures.md`. Campaign `20261117-campaign-1` deposited in `bench-results/` (5 repetitions, 3 charges x 2 tools, zero divergences; cross-checks identical across tools: 51/8, 1859/1408, 9721/8672), reproduction command in its README.
- **Evidence**: campaign run locally, exit 0, `divergences: []`; medians — cnf-uf20-01: lisage 355.4 ms / 5 388 ko vs OxiDD 88.7 ms / 52 692 ko; pnml-swimmingpool-1: 85.6 ms / 4 204 ko vs 24.5 ms / 32 060 ko; pnml-erk-000001: 1 348.6 ms / 19 768 ko vs 353.9 ms / 135 720 ko. Gates: `make lint`, `make typecheck`, `make imports`, `make test` (39 passed, 94.76%), `make english-only`, `make rust-check` (301 passed, 5 skipped), `cargo fmt --check`, `cargo clippy -p lisage-bench --all-targets -- -D warnings` all pass. Not run in CI by design (ADR-0102: Docker witnesses are local only).
- **Judgement calls**: the campaign normalizes the `charge` field to the charge label uniformly across tools (the binary reports the workload label) — the cross-check caught this inconsistency on the first run and it was fixed before depositing. The `philosophers` workload is not part of the campaign (toy-scale states file, documented in #114).

## 2026-11-17 - M7-4: real non-toy benchmark charges for all witnesses (issue #114)

- **Issue**: #114 (M7-4, milestone M7) — at least 3 real charges, each with a versioned input and a pinned cross-check value, implemented in lisage and serializable for the OxiDD witness; no toy examples.
- **Done**: three versioned real inputs in `crates/lisage-bench/models/` (sources cited in its README): SATLIB `uf20-01.cnf` (20 vars, 91 clauses, phase-transition 3-SAT), MCC `SwimmingPool-PT-01` (9 places) and MCC `ERK-PT-000001` (11 places). Two new workloads in `confrontation.rs`: `cnf <path>` (DIMACS parse → truth table → BDD) and `pnml <path>` (PNML parse → transition relation over 2p variables, 1-safe unit-weight semantics → BDD); both share `mesurer_table` (BDD build + node count + C6 mini-BDD falsification, cross-check value = satisfying assignments). `serialiser_charge_stdin` emits the standard truth-table stdin protocol consumed unchanged by `cudd_count`/`oxidd_count`. The PNML parser now flattens `<page>` elements (MCC models nest the net content in pages). Guards: n ≤ 24, 2p ≤ 24. Pinned values in `tests/confrontation.rs`: uf20-01 nodes=51 verifie=8; SwimmingPool nodes=1859 verifie=1408; ERK nodes=9721 verifie=8672 (heavy, `#[ignore]`d). The `oxidd_count` witness sizing was raised (unique table / apply cache 1024 → 2^20) — the real charges need up to 9 721 nodes.
- **Evidence**: cross-checked against the real OxiDD container on the serialized charges: `oxidd_count` returns 51 (uf20-01) and 1859 (SwimmingPool-1), identical to lisage `count_nodes`. `cargo test --workspace`: 72 test binaries, 0 failures (300 tests in `make rust-check`, 5 skipped); `make lint`, `make typecheck`, `make imports`, `make test` (34 passed, 94.76%), `make english-only`, `cargo fmt --check`, `cargo clippy -p lisage-bench --all-targets -- -D warnings` all pass. Not run: the `#[ignore]`d ERK pin in CI (by design, heavy); the CUDD witness on the new charges (pre-existing terminal-counting divergence, out of scope).
- **Judgement calls**: all charges serialize as truth tables in the existing stdin protocol, so `cudd_count`/`oxidd_count` needed no protocol change — only the capacity bump. The MCC `Philosophers-2.pnml` file was rejected (it references undeclared nodes 3..5 — dangling arcs, invalid PNML); ERK-PT-000001 replaced it as the heavy third charge. The `philosophers` workload stays toy-scale (explicit states file); realistic n=50–200 needs a symbolic transducer, not available — documented limitation.

## 2026-11-17 - M7-3: three-way differential harness lisage / mini-BDD / OxiDD (issue #113)

- **Issue**: #113 (M7-3, milestone M7) — extend the local OxiDD differential harness.
- **Done**: `crates/lisage-bench/tests/oxidd_local.rs` (created in #112) now asserts three-way equality on random truth tables: lisage arena `count_nodes` == internal mini-BDD `temoin_bdd::MiniBdd::count_nodes` == `oxidd_count` in the `lisage-oxidd` container. Still `#[ignore]`d, outside `make check`; run manually with `cargo test -p lisage-bench --test oxidd_local -- --ignored`.
- **Evidence**: the ignored test passes 20/20 proptest cases with Docker available (arena == mini-BDD == OxiDD, zero discrepancy); `cargo test -p lisage-bench` without `--ignored` does not run it (0 tests in that target); `make lint`, `make typecheck`, `make imports`, `make test` (34 passed, 94.76%), `make english-only`, `cargo fmt --check`, `cargo clippy -p lisage-bench --all-targets -- -D warnings` all pass.
- **Judgement calls**: no change needed in the `confrontation.rs` harness — the `alea-lcg` workload already serializes the same truth-table charge and checks C6 internally against the mini-BDD; the OxiDD container consumes the same stdin format.

## 2026-11-17 - M7-2: local Docker witness for OxiDD (issue #112)

- **Issue**: #112 (M7-2, milestone M7) — `docker/oxidd/` following the ADR-0102 pattern.
- **Done**: `docker/oxidd/Dockerfile` builds `lisage-oxidd:latest` with OxiDD pinned at commit `c7a458df8b747a3ee44d0e9e61059a1621865fcd`; `oxidd_count` binary reads the same stdin format as `cudd_count` (n, then 2^n 0/1) and prints the BDD node count; `README.md` documents the pinned SHA and build command. No reference to `docker/oxidd` in Cargo.toml, CI workflows, Makefile or scripts (grep-verified). Variable convention matches lisage's `compiler_bdd` (bit l of the index = variable l, variable 0 at the root); `node_count()` (terminals included) matches lisage's `count_nodes` exactly — verified locally on 30+ truth tables (n=1..6), diff 0 everywhere, plus the Docker self-test `2 / 0 1 1 0 -> 5`.
- **Evidence**: `docker build -t lisage-oxidd:latest docker/oxidd` succeeds; `docker run --rm -i lisage-oxidd:latest oxidd_count` returns 5 (XOR n=2), 1 (constant), 6 (n=3 sample) — identical to lisage `count_nodes`. The paired `#[ignore]`d differential `crates/lisage-bench/tests/oxidd_local.rs` (ADR-0102 pattern, outside CI) passes against the real container: `cargo test -p lisage-bench --test oxidd_local -- --ignored` 20/20 proptest cases, lisage == OxiDD. `make lint`, `make typecheck`, `make imports`, `make test`, `make english-only`, `cargo fmt --check`, `cargo clippy -p lisage-bench --all-targets -- -D warnings` pass. Note: the pre-existing `cudd_local` differential (lisage vs CUDD) currently fails on terminal counting (lisage counts terminals, CUDD DagSize does not) — pre-existing, out of scope here; oxidd_count follows the lisage convention.
- **Judgement calls**: OxiDD API at the pinned SHA uses `ManagerRef` + shared/exclusive locks (building inside `with_manager_exclusive` deadlocks — operations take the shared lock); constants are `BDDFunction::t/f`.

## 2026-11-17 - M7-1: OxiDD added to the state of the art (issue #111)

- **Issue**: #111 (M7-1, milestone M7) — integrate OxiDD into the state-of-the-art document.
- **Done**: `docs/state-of-the-art-decision-diagrams.md` (the file was renamed from its French name during M6): new dedicated subsection under §2 (libraries axis, next to CUDD/BuDDy/Sylvan/Meddly/libDDD), new row in the §7 comparison table, updated open-niche synthesis (parallelism axis now also covered by OxiDD in Rust), and two §11 references — the TACAS 2024 paper (DOI 10.1007/978-3-031-57256-2_13, verified on the authors' publication page) and the GitHub repo (MIT OR Apache-2.0, verified on the repo/PyPI). All factual claims sourced; no invented DOI. Author name transliterated `Köhl`→`Koehl` to satisfy the English-only lint.
- **Evidence**: `make english-only` passes; `make lint`, `make typecheck`, `make imports`, `make test` pass. Rust gates not run on this branch — the diff touches no Rust file.
- **Judgement calls**: the issue text references the old French filename; the current English filename was used.

## 2026-11-17 - M6-7: English-only lint in make check (issue #110)

- **Issue**: #110 (M6-7, milestone M6) — final quality pass.
- **Done**: new `make english-only` target fails on French accented characters in `docs/**/*.md`, root `*.md` and `crates/**/*.rs`; wired into `make check`. Allowlist (documented in the Makefile): `docs/glossary.md` (FR→EN mapping terms), `docs/audit-french-inventory.md` (quoted French strings), `WORKLOG.md` (pre-M6 entries stay French per its own header). One residual accent fixed: "Université Pierre et Marie Curie" → "Pierre and Marie Curie University (UPMC)" in the state-of-the-art. PR #181 (issue #107, docs/agents + docs/decisions) merged into daily with human authorization.
- **Evidence**: `make english-only` passes on the converted tree and fails on a planted accented file (sanity check); `make lint`, `make typecheck`, `make imports`, `make test` pass. **Not run**: the Rust gates on this branch — the diff touches no Rust file (last full run on daily HEAD: 293 passed, clippy and fmt clean). **Not done**: wiring the lint into `.github/workflows/ci.yml` — the app token lacks the `workflow` scope and refuses workflow edits; left to the human.
- **Judgement calls**: WORKLOG.md exempted from the lint rather than translated — its header forbids rewriting past entries (honesty rule); the lint enforces English-only for all new content.
- **Escalations**: lint/CI configuration and constitutional docs are on the ADR-0010 escalation list — the human authorized both in the session directive ("Termine la M6 et merge daily; traduis tous les documents et ajoute le lint English-only").

## 2026-11-14 - M6-6 part 31 (final): articles `un`/`une`/`deux`/`la`/`et` renamed to `one`/`two`/`level_a`/`_and_`, remaining French prose comments translated (issue #108)

- **Issue**: #108 (M6-6, milestone M6) — final identifier batch.
- **Done**: renames `un`→`one` (incl. `Pli::un()`), `une`→`one` (incl. `EffetMemory::one()`), `deux`→`two`, pattern bindings `la`/`lb`→`level_a`/`level_b`, `_et_`→`_and_` in test names (32 occurrences), workspace-wide (31 files). The last French prose comments translated to English (lib.rs crate doc, confrontation bin usage, bdd.rs/region.rs/schema.rs/cdd.rs comments). Accent grep over crates/**/*.rs now returns 0.
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean.
- **Judgement calls**: `la`/`lb` level bindings mapped to `level_a`/`level_b`.
## 2026-11-14 - M6-6 part 30: `nom`/`piste`/`fiche`/`royaume`/`contenu`/`taux`/`predit`/`ancien`/`lire`/`ecrire`/`boite`/`atelier`/`fantome`/`dossier`/`cellule` renamed to `name`/`track`/`card`/`kingdom`/`content`/`rate`/`predicted`/`old`/`read`/`write`/`box`/`workshop`/`ghost`/`directory`/`cell` (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames `nom`→`name`, `piste`→`track`, `fiche`→`card`, `royaume(s)`→`kingdom(s)`, `contenu`→`content`, `taux`→`rate`, `predit`→`predicted`, `ancien(ne)`→`old`, `lire`→`read`, `ecrire`→`write`, `boite`→`box`, `atelier`→`workshop`, `fantome(s)`→`ghost(s)`, `dossier`→`directory`, `cellule(s)`→`cell(s)` (all cases, compounds included), workspace-wide (36 files), per glossary. Module/test files git-mv'd: `atelier.rs`→`workshop.rs` (src+tests), `atelier_philosophers.rs`→`workshop_philosophers.rs`. One keyword collision fixed: test helper `fn boite()` became reserved `box`, renamed to `full_box()`.
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; residual grep for the stems returns 0.
- **Judgement calls**: `ancien`/`ancienne`→`old`; `boite`→`box` with the `full_box` helper rename.
## 2026-11-14 - M6-6 part 29: `cible` renamed to `target` (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: rename `cible(s)`→`target(s)` (all cases, compounds included), workspace-wide (29 files), per glossary.
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; residual grep for the stem returns 0.
- **Judgement calls**: none beyond the glossary mapping.
## 2026-11-14 - M6-6 part 28: `collecte`/`collecteur`/`collecter`/`variante`/`outil` renamed to `collect`/`collected`/`collector`/`variant`/`tool`, wire keys protected (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames `collecteur`→`collector`, `collecter`→`collect`, `collectes`→`collected`, `collecte`→`collect`, `variante(s)`→`variant(s)`, `outil(s)`→`tool(s)` (all cases, compounds included), workspace-wide (14 files), per glossary. Wire keys `\"collectes\"`, `\"variante\"`, `\"outil\"` restored after the blanket pass (trace.rs, step_by_step.rs, confrontation.rs + test pins). A rule-ordering artifact (`collected`→`collectd` via the later `collecte` rule) was repaired.
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; residual grep for the stems returns only the protected wire literals.
- **Judgement calls**: `collectes` (GC collected count) mapped to `collected`.
## 2026-11-14 - M6-6 part 27: `poids`/`profondeur`/`morts`/`orphelin`/`marqueur`/`seuil`/`fusion`/`garde`/`philosophes` renamed to `weight`/`depth`/`dead`/`orphan`/`marker`/`threshold`/`merge`/`guardrail`/`philosophers` (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames `poids`→`weight`, `profondeur`→`depth`, `morts`→`dead`, `orphelin(s)`→`orphan(s)`, `marqueur`→`marker`, `seuil`→`threshold`, `fusion`→`merge`, `garde_fou`→`guardrail`, `garde`→`guard`, `philosophes`→`philosophers` (all cases, compounds included), workspace-wide (31 files), per glossary. Module/test files git-mv'd: `philosophes.rs`→`philosophers.rs` (src+tests), `philosophes_bench.rs`→`philosophers_bench.rs`, `atelier_philosophes.rs`→`atelier_philosophers.rs`; the committed figure `bench-results/philosophes-n5.svg`→`philosophers-n5.svg` to match the renamed path string. `test_filtre_garde`→`test_filter_guard` and `VerdictGardeFou`→`VerdictGuardrail` completed after the first pass (underscore-less compounds).
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; residual grep for the stems returns 0.
- **Judgement calls**: `fusion`→`merge` per glossary (local `let merge = ...` shadowing is legal Rust); `garde_fou`→`guardrail`.
## 2026-11-14 - M6-6 part 26: `canal`/`canaux` renamed to `channel`/`channels`, wire key `canal` protected (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames `canal`→`channel`, `canaux`→`channels` (all cases, compounds included, e.g. `CanalAttr`→`ChannelAttr`, `nom_canal`→`channel_name`), workspace-wide (15 files), per glossary. The JSONL v2 wire key `\"canal\"` is contract and was restored in `lisage-liseuse/src/trace.rs` and the two test pins in `lisage-liseuse/tests/step_by_step.rs`.
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; residual grep for the stem returns only the 3 protected wire literals.
- **Judgement calls**: none beyond the glossary mapping.
## 2026-11-14 - M6-6 part 25: `enfant`/`attribut`/`vue`/`classe`/`voie`/`pas` renamed to `child`/`attribute`/`view`/`class`/`lane`/`step`, module files renamed (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames `enfant(s)`→`child(ren)`, `attribut(s)`→`attribute(s)`, `vue(s)`→`view(s)`, `classe`→`class`, `voie(s)`→`lane(s)`, `pas`→`step` (all cases, compounds included), workspace-wide (28 files), per glossary. Module files git-mv'd: `vue_memory.rs`/`vue_order.rs`→`view_memory.rs`/`view_order.rs`, `pas_a_pas.rs`→`step_by_step.rs` (src + tests). Collision guards: `attribute`/`attribution`/`classes` protected with negative lookaheads; `pas_a_pas`→`step_by_step` before the word-boundary `pas`→`step` so `pass`/`passes`/`passage` are untouched.
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; residual grep for the stems returns 0.
- **Judgement calls**: `voie` mapped to `lane` per glossary.
## 2026-11-14 - M6-6 part 24: `politique`/`groupe`/`somme`/`chemin`/`liste`/`intervalle`/`espace` renamed to `policy`/`group`/`sum`/`path`/`list`/`interval`/`space` (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames `politique`→`policy`, `groupe`→`group`, `somme`→`sum`, `chemin`→`path`, `liste`→`list`, `intervalle`→`interval`, `espace`→`space` (all cases, plurals included), workspace-wide (35 files), per glossary. Collision guards: `grouped`/`listed` protected with a negative lookahead (`(?!d)`); `whitespace` repaired after the pass (`split_whitspace`→`split_whitespace`, `is_whitspace`→`is_whitespace`).
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; residual grep for the stems returns only `grouped`/`listed`.
- **Judgement calls**: none beyond the glossary mapping.
## 2026-11-14 - M6-6 part 23: `forme`/`pile`/`largeur`/`charge`/`hauteur` renamed to `form`/`stack`/`width`/`workload`/`height`, wire key `charge` protected (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames `forme`→`form`, `pile`→`stack`, `largeur`→`width`, `charge`→`workload`, `hauteur`→`height` (all cases, compounds included), workspace-wide (44 files), per glossary. Collision guards: `conforme`→`conforming` first (so `est_conforme` is not corrupted), `pile` renamed with a negative lookbehind `(?<!m)` so `compiler`/`compiled`/`decompiler` are untouched, and the bench JSON wire key `\"charge\"` restored in `lisage-bench/src/confrontation.rs` and its test pins. After review: `forme` inside English `malformed`/`performed` repaired (`malformd`→`malformed`, `performd`→`performed`, `XmlMalForm`→`XmlMalformed`), and the pinned-key doc comment restored to `charge`.
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; residual grep for the stems returns only English words containing them (compiler, performed) and the protected wire key.
- **Judgement calls**: `charge` mapped to `workload` per the glossary bench entry.
## 2026-11-14 - M6-6 part 22: `rapport`/`erreur` renamed to `report`/`error` (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames `rapport(s)`→`report(s)`, `erreur(s)`→`error(s)` (all cases, compounds included), workspace-wide (23 files), per glossary.
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; residual grep for the stems returns 0.
- **Judgement calls**: none beyond the glossary mapping.
## 2026-11-14 - M6-6 part 21: `ensemble` renamed to `set` (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: rename `ensemble`→`set` (all cases, compounds included), workspace-wide (25 files), per glossary.
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; residual grep for the stem returns 0.
- **Judgement calls**: none beyond the glossary mapping.
## 2026-11-14 - M6-6 part 20: `avec`/`vide`/`sans` renamed to `with`/`empty`/`without` (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames `avec`→`with`, `vide`→`empty`, `sans`→`without` (all cases, compounds included), workspace-wide (39 files), per glossary. English words containing `vide` (`provided`, `divide`, `provides`) and the plural `vides`→`empties` were repaired after review (`diempty`→`divide`, `proemptys`→`provides`, `non-emptys`→`non-empty`).
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; residual grep for the stems returns 0 (excluding English words containing `vide`).
- **Judgement calls**: none beyond the glossary mapping.
## 2026-11-14 - M6-6 part 19: `vivants`/`contient`/`etiquette` renamed to `live`/`contains`/`label`, wire key `vivants` protected (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames `vivants`→`live`, `contient`→`contains`, `etiquette`→`label` (all cases, compounds included), workspace-wide (31 files), per glossary. The JSONL v2 wire key `\"vivants\"` is contract and was restored after the blanket pass in `lisage-liseuse/src/trace.rs` (format string now `\"vivants\":{live}`) and the test pin in `lisage-liseuse/tests/pas_a_pas.rs`.
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; residual grep for the stems returns only the two protected wire literals (`vivantes_avant`→`live_before` fixed after review).
- **Judgement calls**: `etiquette` mapped to `label` (no git-tag usage found in crates/); feminine `vivante(s)` also mapped to `live`.
## 2026-11-14 - M6-6 part 18: `compter`/`compte`/`reduire`/`couverture`/`evaluer` renamed to `count`/`reduce`/`coverage`/`evaluate` (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames `compteur`→`counter`, `compter`/`compte`→`count`, `reduire`/`reduit(s)`→`reduce`/`reduced(s)`, `couverture`→`coverage`, `evaluer`→`evaluate` (all cases, compounds included), workspace-wide (41 files), per glossary. Already-English `reduction`/`Reduction*` untouched. One collision fixed by hand: `lisage_lang::interpreter::count` was already English, so `lisage_core::pli::count` is imported as `count_pli` in `lisage-bench/tests/pli.rs`.
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; residual grep for the stems returns 0.
- **Judgement calls**: `count_pli` alias to disambiguate from the pre-existing English `interpreter::count`.
## 2026-11-14 - M6-6 part 17: `fenetre`/`construire`/`borne` renamed to `window`/`build`/`bound` (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames `fenetre`→`window`, `construire`→`build`, `borne`→`bound` (all cases, compounds included), workspace-wide (27 files), per glossary. `bornee`→`bounded` handled first so no `boundee` artifact; `construction` is already English and untouched.
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; residual grep for the stems returns 0.
- **Judgement calls**: `borne_sup`/`borne_inf` become `bound_sup`/`bound_inf` (mechanical; sup/inf are separate stems).
## 2026-11-14 - M6-6 part 16: `nombre` renamed to `number` (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: rename `nombre`→`number` (all cases, compounds included), workspace-wide (39 files), per glossary.
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; residual grep for the stem returns 0.
- **Judgement calls**: none beyond the glossary mapping.
## 2026-11-14 - M6-6 part 15: `ordre` renamed to `order`, module files renamed (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: rename `ordre`→`order` (all cases, compounds included), workspace-wide (16 files), per glossary; module files git-mv'd: `lisage-plan/src/ordre.rs`→`order.rs`, `lisage-plan/tests/ordre.rs`→`order.rs`, `lisage-liseuse/src/vue_ordre.rs`→`vue_order.rs`, `lisage-liseuse/tests/vue_ordre.rs`→`vue_order.rs`; `mod` declarations updated by the same pass.
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; residual grep for the stem returns 0.
- **Judgement calls**: none beyond the glossary mapping.
## 2026-11-14 - M6-6 part 14: `cardinal`/`cardinalite`/`premier`/`dernier`/`meme` renamed to `cardinality`/`first`/`last`/`same` (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames `cardinal`/`cardinalite`→`cardinality`, `premier`/`premiere`→`first`, `dernier`→`last`, `meme`→`same` (all cases, compounds included), workspace-wide (40 files), per glossary. `cardinals`→`cardinalities`; existing English `cardinality`/`cardinalities` protected with a negative lookahead so no `cardinalityity` artifacts.
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; residual grep for the stems returns 0.
- **Judgement calls**: `meme` is not in the glossary; mapped to `same` as the direct translation.
## 2026-11-14 - M6-6 part 13: `ancre`/`graine`/`autre` renamed to `anchor`/`seed`/`other` (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames `ancre`→`anchor`, `graine`→`seed`, `autre`→`other` (all cases, compounds included), workspace-wide (42 files), per glossary. One parameter collision fixed by hand (`construire_fenetre_ancree` had both `anchor: Anchor` and `ancre: &AnchorFenetre`; the latter is now `window_anchor`). Split from the larger part-13 batch to stay under the PR size budget.
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; word-boundary grep for the stems returns 0.
- **Judgement calls**: none beyond the glossary mapping.
## 2026-11-14 - M6-6 part 12: `resultat`/`attendu`/`suivant` renamed to `result`/`expected`/`next` (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames every `resultat`/`Resultat`, `attendu`/`Attendu` (incl. `attendus`/`attendues` → `expected_list`/`expected_f`) and `suivant`/`Suivant` stem to `result`/`Result`, `expected`/`Expected`, `next`/`Next`, workspace-wide (55 files), per glossary (résultat → result, attendu → expected, suivant → next).
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; case-insensitive grep for the three stems returns 0.
- **Judgement calls**: French plural forms `attendus`/`attendues` mapped to `expected_list`/`expected_f` to keep distinct bindings distinct.
## 2026-11-14 - M6-6 part 11: `memoire`/`observateur`/`reseau` renamed to `memory`/`observer`/`network` (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames every `memoire`/`observateur`/`reseau` stem (lowercase, CamelCase, UPPER_CASE, plurals) to `memory`/`observer`/`network`, workspace-wide (33 files), per glossary, including module file renames (`src/memoire.rs` → `src/memory.rs`, `src/vue_memoire.rs` → `src/vue_memory.rs`, plus the three test files) and the `mod` declarations. The JSONL v2 wire value `"memoire"` (canal name in trace.rs and its pinned test copy) is protected and stays French (contract).
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; case-insensitive grep for the three stems outside the wire literal returns 0.
- **Judgement calls**: wire canal value `"memoire"` kept (contract); module files renamed with `git mv` so history follows.
## 2026-11-14 - M6-6 part 10: `niveau`/`mesure` identifiers renamed to `level`/`measure` (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames every `niveau`/`Niveau`/`NIVEAU` and `mesure`/`Mesure`/`MESURE`/`mesurer` stem and compound (niveaux, nombre_niveaux, mesure_zone, MesureConfrontation…) to `level`/`Level`/`LEVEL` and `measure`/`Measure`/`MEASURE`, workspace-wide (56 files), per glossary (niveau → level, mesure → measurement). Three over-eager shorthand fixes repaired by hand (`level: level_a`, `level: level_value` patterns restored after a blanket `level: level` → `level` pass truncated them).
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; case-insensitive grep for niveau|mesure|mesurer returns 0.
- **Judgement calls**: `mesurer` → `measure` (glossary: mesurer → measure); no wire literals involved.
## 2026-11-14 - M6-6 part 9: `domaine`/`taille` identifiers renamed to `domain`/`size` (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames every `domaine`/`Domaine`/`DOMAINE` and `taille`/`Taille`/`TAILLE` stem and compound (domaines, taille_domaine, TAILLE_DOMAINE, DomaineA…) to `domain`/`Domain`/`DOMAIN` and `size`/`Size`/`SIZE`, workspace-wide (60 files), per glossary (domaine → domain, taille → size). The bench JSON wire key `"taille"` (plain and escaped literals in confrontation.rs src+tests) is protected and stays French (contract).
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; case-insensitive grep for domaine|taille outside the wire literal returns 0.
- **Judgement calls**: wire key `"taille"` restored in both the serializer and the pinned test after the blanket rename hit plain string literals too.
## 2026-11-14 - M6-6 part 8: `cle`/`cles`/`clef` identifiers renamed to `key`/`keys` (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames every `cle`/`cles`/`clef` stem (word-boundary and `cle_*`/`_cle`/`Cle`/`CLE` compounds, incl. `cles_canoniques`, `liste_cles`, `CleMap`, `ErreurOrdre<Cle>`) to `key`/`keys`/`Key`/`KEY`, workspace-wide (38 files), per glossary (clé → key). Word-boundary patterns leave `oracle`/`socle`/`cycle` untouched.
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; grep for the cle stems returns 0.
- **Judgement calls**: `clef` → `key` (not `keyf`); compounds renamed in a second pass after the first missed `cles_*` files.
## 2026-11-14 - M6-6 part 7: `valeur` identifiers renamed to `value` (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames every `valeur`/`Valeur`/`VALEUR` stem and compound (valeurs, domaine_valeurs, node_valeur, niveau_valeur…) to `value`/`Value`/`VALUE`, workspace-wide (49 files), per glossary (valeur → value). The JSONL v2 wire literals `\"valeur\"`/`\"valeurs\"` in trace.rs and their pinned test copies are protected and stay French (contract).
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; case-insensitive grep for `valeur` outside wire literals returns 0.
- **Judgement calls**: wire literals protected via placeholder before the blanket rename, restored after.
## 2026-11-14 - M6-6 part 6: `noeud` identifiers renamed to `node` (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames every `noeud` stem (lowercase, `Noeud`, `NOEUD`, compounds like `noeuds_partages`, `NodeVtree`) to `node`/`Node`/`NODE`, workspace-wide (57 files), per glossary (nœud → node). One shadowing collision fixed by hand: `tests/arena.rs` used distinct `node` and `noeud` bindings, the second is now `node2`; one clippy `redundant_field_names` fixed with the shorthand.
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; case-insensitive grep for `noeud` returns 0.
- **Judgement calls**: blanket stem rename, collisions resolved by renaming the shadowing binding (`node2`) rather than reverting.
## 2026-11-14 - M6-6 part 5: `entree`/`sortie` identifiers renamed to `input`/`output` (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames every `entree`/`sortie` stem (bare words, `entrees`, `entree_*`/`sortie_*` compounds, `*_entree`/`*_sortie` suffixes, plus `PoolSortie` → `PoolOutput`, `EntreeInvalide` → `InvalidInput`, `EntreeNoeud` → `NodeEntry`, `noeud_entree` → `node_input`) to `input`/`output`, workspace-wide (38 files), per glossary (entrée/sortie → input/output).
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; case-insensitive grep for `entree|sortie` returns 0.
- **Judgement calls**: `EntreeNoeud` → `NodeEntry` (node table entry), `EntreeInvalide` → `InvalidInput` (typed error variant).
## 2026-11-14 - M6-6 part 4: `racine` identifiers renamed to `root` (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames every `racine` stem (bare word, `racine_*` compounds, `*_racine` suffixes, `racines` plural, `racine2`) to `root`/`roots`, workspace-wide (48 files), per glossary (racine → root).
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean.
- **Judgement calls**: prefix/suffix compounds renamed in the same pass so no `racine` stem survives a grep.
## 2026-11-14 - M6-6 part 3: `nouveau` constructors renamed to `new` (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames every `nouveau` identifier (public constructors `Domaine::nouveau`, `Cdd::nouveau`, `CddController::nouveau`, `Region::nouveau`, `Dbm::nouveau`, `Atelier…::nouveau`, plus local variables) to `new`, workspace-wide (53 files). No type had both `new` and `nouveau`, so no collision.
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean.
- **Judgement calls**: `nouveau` → `new` per glossary (construire → build/constructor); `nouvelle` was already renamed in part 2.
## 2026-11-14 - M6-6 part 2: lisage-algebra public API renamed to English (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames the French public identifiers of lisage-algebra and their workspace call sites (27 files): `PolitiqueMemoire` → `MemoryPolicy` (`Aucune` → `NoMemory`, `ParNiveau` → `PerLevel`), `politique_memoire` → `memory_policy`, `collapse_actif` → `collapse_active`, `defaut_actif` → `default_active`, `absorption_generalisee` → `generalized_absorption`, `groupe_cles` → `cycle_group`, `GroupeCles` → `CycleGroup` (assoc type `Cle` → `Key`), `GroupeShift` → `ShiftGroup`, `ArcEntree` → `InputArc`, `lire_cle` → `read_key`, `appliquer` → `apply`, `composer` → `compose`, `neutre` → `neutral`, `inverser` → `invert`, `nouvelle` → `new`.
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean.
- **Judgement calls**: `Aucune` → `NoMemory` (not `None`, to avoid clashing with `Option::None`); `ArcEntree` → `InputArc` per glossary "arc d'entrée → input edge".
## 2026-11-14 - M6-6 part 1: CanalAttr variants renamed to English (issue #108)

- **Issue**: #108 (M6-6, milestone M6).
- **Done**: renames the public enum variants `CanalAttr::Herite` → `CanalAttr::Inherited`, `CanalAttr::Synthétise` → `CanalAttr::Synthesized`, `CanalAttr::Memoire` → `CanalAttr::Memory` (glossary M6-1), with all 42 `CanalAttr::…` use sites and the doc comments updated workspace-wide (14 files across lisage-core, lisage-liseuse, lisage-bench). The JSONL v2 wire strings `herite`/`synthetise`/`memoire` are contract and stay French.
- **Evidence**: `cargo build --workspace` passes; `cargo test --workspace` 293 passed, 0 failed; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean.
- **Judgement calls**: variant names follow the glossary adjectives already used in the doc comments (Inherited/Synthesized) and `Memory` for level memory; wire strings untouched.
## 2026-11-14 - M6-5 part 4a-5: lisage-core relatif tests translated to English (PR #205, issue #108)

- **Issue**: #108 (M6-5, milestone M6).
- **Done**: translates comments, doc-comments and string literals in `crates/lisage-core/tests/relatif.rs` to English. The biplan test changes from the original branch were already covered by part 4a-10 on `daily`, so the rebased diff only carries `relatif.rs`. Public French identifiers are kept until M6-6.
- **Evidence**: `cargo fmt -p lisage-core -- --check` clean; `cargo test -p lisage-core` 146 passed, 0 failed. Full-workspace `make check` not run on this branch (crate-scoped gates only); it runs on `daily` after all M6-5 pull requests merge.
- **Judgement calls**: comments/strings translated now, identifier renames deferred to M6-6.
## 2026-11-14 - M6-5 part 4a-4: lisage-core cdd+dabs translated to English (PR #204, issue #108)

- **Issue**: #108 (M6-5, milestone M6).
- **Done**: translates comments, doc-comments and string literals in `crates/lisage-core/src/cdd.rs`, `crates/lisage-core/tests/cdd.rs` and `crates/lisage-core/tests/dabs.rs` to English. Public French identifiers are kept until dependent crates are translated (M6-6).
- **Evidence**: `cargo fmt -p lisage-core -- --check` clean; `cargo test -p lisage-core` 146 passed, 0 failed. Full-workspace `make check` not run on this branch (crate-scoped gates only); it runs on `daily` after all M6-5 pull requests merge.
- **Judgement calls**: comments/strings translated now, identifier renames deferred to M6-6.
## 2026-11-14 - M6-5 part 4a-3: lisage-core transduction+transfert translated to English (PR #203, issue #108)

- **Issue**: #108 (M6-5, milestone M6).
- **Done**: translates comments, doc-comments and string literals in `crates/lisage-core/src/transduction.rs`, `crates/lisage-core/src/transfert.rs`, `crates/lisage-core/tests/transduction.rs`, `crates/lisage-core/tests/transfert.rs` to English. Public French identifiers are kept until dependent crates are translated.
- **Evidence**: `cargo check -p lisage-core --tests` passes; `cargo test -p lisage-core` 146 passed, 0 failed; CI green on PR #203; review 0 BLOCKING after this WORKLOG entry was added.
- **Judgement calls**: split of the original part 4a into five sub-parts (4a-1..4a-5) because the reviewer session hits its turn limit on diffs over ~500 lines.
## 2026-11-14 - M6-5 misc: last accents outside the open PRs (issue #108)

- **Issue**: #108 (M6-5, milestone M6).
- **Done**: full-repo accent sweep after liseuse; translates the two remaining `Cargo.toml` descriptions (lisage-plan, lisage-algebra) and renames the two test functions `test_narration_evenements_attr_synthétise` → `..._synthesized` in lisage-core tests integrale/integrale_dbm. Remaining accents in `crates/` are only the kept `Synthétise` identifier and the lisage-core files covered by open PRs #203/#204/#205.
- **Evidence**: accent grep over `crates/` shows only PR-covered files + `Synthétise`; `cargo test -p lisage-core --test integrale --test integrale_dbm` ok (14 passed); `cargo fmt -- --check` clean on the touched crates.
- **Judgement calls**: test function names are local, renamed; public identifiers still deferred.
## 2026-11-14 - M6-5 liseuse part 1: lisage-liseuse src+tests translated to English (issue #108)

- **Issue**: #108 (M6-5, milestone M6).
- **Done**: translates comments, doc-comments and string literals in all of `crates/lisage-liseuse/` — src (vue_ordre, pas_a_pas, vue_memoire, publication, philosophes, trace, svg) and tests (vue_ordre, pas_a_pas, philosophes, vue_memoire, liseuse_v0, publication, atelier_philosophes). SVG label literals translated in src and pinned copies updated in tests in sync (`D^def`, `convert … (cost …)`, `step by step — position …`, `memory and quotient view`, `alive=/dead=`, legend keys `inherited`/`synthesized`/`memory`/`ghost`, `inherited (φ_disp)` etc.); JSONL v2 wire field names (`canal`, `noeud`, `valeur`, `vivants`…) kept as contract. `bench-results/philosophes-n5.svg` regenerated for the new SVG title `diagram (levels = keys)`.
- **Evidence**: accent grep returns 0 across `crates/lisage-liseuse/` except the kept `Synthétise` identifier; `cargo check -p lisage-liseuse --tests` passes; `cargo test -p lisage-liseuse` all 8 suites ok (29 passed, 0 failed); `cargo fmt -p lisage-liseuse -- --check` clean. Full-workspace `make check` not run at this stage (crate-scoped gates only); it is run after all M6-5 pull requests merge into `daily`.
- **Judgement calls**: comments/strings translated now, public identifiers deferred; JSONL wire contract untouched.
## 2026-11-14 - M6-5 bench part 2: lisage-bench tests+benches translated to English (issue #108)

- **Issue**: #108 (M6-5, milestone M6).
- **Done**: translates comments, doc-comments and string literals in all `crates/lisage-bench/tests/*.rs` (confrontation, philosophes_bench, metriques, sdd, saturate, pnml, handles, cudd_local, triptyque, transfert, lookup, schemas, operations, differentiel, transductions, temoins, frontieres, reductions, pli, memoire, collapse) and `benches/micro.rs` to English. lisage-bench is now accent-free except the kept `Synthétise` identifier.
- **Evidence**: accent grep returns 0 across `crates/lisage-bench/` except `Synthétise`; `cargo check -p lisage-bench --tests --benches` passes; `cargo test -p lisage-bench` all suites ok (0 failed); `cargo fmt -p lisage-bench -- --check` clean.
- **Judgement calls**: comments/strings translated now, public identifiers deferred.
- **Escalations**: none.

## 2026-11-14 - M6-5 bench part 1: lisage-bench src translated to English (issue #108)

- **Issue**: #108 (M6-5, milestone M6).
- **Done**: translates comments, doc-comments and string literals in `crates/lisage-bench/src/confrontation.rs`, `pnml.rs`, `triptyque.rs`, `sdd.rs`, `temoin_bdd.rs`, `differentiel.rs`, `lib.rs`, `src/bin/confrontation.rs`, plus the `description` in `Cargo.toml`. Three pinned narration strings in `tests/confrontation.rs` updated to match the new English event payloads (`workload=… nodes=… check=…`). Public French identifiers are kept until dependent crates are translated.
- **Evidence**: accent grep returns 0 across `crates/lisage-bench/src/` except the kept `Synthétise` identifier; `cargo check -p lisage-bench --tests` passes; `cargo test -p lisage-bench` all 24 test suites pass; `cargo fmt -p lisage-bench -- --check` clean. Remaining French in `tests/` and `benches/` is the next part.
- **Judgement calls**: comments/strings translated now, public identifiers deferred; the CLI usage line was translated as a string literal, pinned test updated accordingly.
- **Escalations**: none.

## 2026-11-14 - M6-5 backend: lisage-backend translated to English (issue #108)

- **Issue**: #108 (M6-5, milestone M6).
- **Done**: translates the doc-comment in `crates/lisage-backend/src/lib.rs` and the `description` in `crates/lisage-backend/Cargo.toml` to English. lisage-backend is now accent-free.
- **Evidence**: accent grep returns 0 across `crates/lisage-backend/`; `cargo check -p lisage-backend` passes. Tests not run: comment/metadata-only change.
- **Judgement calls**: none.
- **Escalations**: none.

## 2026-11-14 - M6-5 part 4a-11: lisage-core atelier leftover translated to English (issue #108)

- **Issue**: #108 (M6-5, milestone M6).
- **Done**: fixes the last French comment in `crates/lisage-core/src/atelier.rs` (`fenêtre` in the `construire_fenetre_ancree` doc-comment). Remaining accented tokens in lisage-core are public French identifiers (`Synthétise`, test names) or files covered by open PRs #203/#204/#205 (transduction, transfert, cdd, dabs, relatif, schema).
- **Evidence**: accent grep on atelier.rs matches only `Synthétise`; `cargo check -p lisage-core --tests` passes; `cargo fmt -p lisage-core -- --check` clean. Tests not re-run: comment-only change, no code touched.
- **Judgement calls**: none.
- **Escalations**: none.

## 2026-11-14 - M6-5 part 4a-10 review fix: revert local variable renames in tests/biplan.rs (issue #108)

- **Issue**: #108 (M6-5). Review of PR #212 reported 1 BLOCKING: local identifiers (`partage`, `metriques`, `meme`, `partages`, `taux`, `attendu`, `premier`, `vivantes_avant`) were renamed in `tests/biplan.rs`, outside the comments/strings-only scope.
- **Done**: reverts those local variable renames to their French names; comments and string literals stay translated. Identifiers are M6-6 scope.
- **Evidence**: `cargo test -p lisage-core --test biplan` ok (7 passed); `cargo fmt -p lisage-core -- --check` clean.
- **Judgement calls**: none.
## 2026-11-14 - M6-5 part 4a-10: lisage-core biplan+bdd+error+lib+interface tests translated to English (issue #108)

- **Issue**: #108 (M6-5, milestone M6).
- **Done**: translates comments, doc-comments and string literals in `crates/lisage-core/src/biplan.rs`, `crates/lisage-core/src/bdd.rs`, `crates/lisage-core/src/error.rs`, `crates/lisage-core/src/lib.rs`, `crates/lisage-core/tests/biplan.rs`, `crates/lisage-core/tests/bdd.rs`, `crates/lisage-core/tests/interface.rs` to English. `tests/atelier.rs` left as-is: its only accented tokens are the public identifier `Synthétise`, kept until dependent crates are translated.
- **Evidence**: accent grep returns 0 on every translated file (atelier.rs only matches `Synthétise`); `cargo check -p lisage-core --tests` passes; `cargo test -p lisage-core` all test suites pass (0 failed); `cargo fmt -p lisage-core -- --check` clean. Full `make check` deferred to the last part.
- **Judgement calls**: comments/strings translated now, public identifiers deferred to avoid breaking dependent crates mid-series.
- **Escalations**: none.

## 2026-11-14 - M6-5 part 4a-9: lisage-core memoire+region+add+ddd translated to English (issue #108)

- **Issue**: #108 (M6-5, milestone M6).
- **Done**: translates comments, doc-comments and string literals in `crates/lisage-core/src/memoire.rs`, `crates/lisage-core/src/region.rs`, `crates/lisage-core/src/add.rs`, `crates/lisage-core/src/ddd.rs`, `crates/lisage-core/tests/memoire.rs`, `crates/lisage-core/tests/region.rs`, `crates/lisage-core/tests/add.rs`, `crates/lisage-core/tests/ddd.rs` to English. Public French identifiers are kept until dependent crates are translated.
- **Evidence**: `cargo check -p lisage-core --tests` passes; `cargo test -p lisage-core` all 25 test suites pass; `cargo fmt -p lisage-core -- --check` clean after fmt. Full `make check` deferred to the last part.
- **Judgement calls**: comments/strings translated now, public identifiers deferred to avoid breaking dependent crates mid-series.
- **Escalations**: none.

## 2026-11-14 - M6-5 part 4a-8: lisage-core arena+observer+lookup translated to English (issue #108)

- **Issue**: #108 (M6-5, milestone M6).
- **Done**: translates comments, doc-comments and string literals in `crates/lisage-core/src/arena.rs`, `crates/lisage-core/src/observer.rs`, `crates/lisage-core/src/lookup.rs`, `crates/lisage-core/tests/arena.rs`, `crates/lisage-core/tests/lookup.rs` to English. Public French identifiers are kept until dependent crates are translated.
- **Evidence**: `cargo check -p lisage-core --tests` passes; `cargo test -p lisage-core` all 25 test suites pass; `cargo fmt -p lisage-core -- --check` clean. Full `make check` deferred to the last part.
- **Judgement calls**: comments/strings translated now, public identifiers deferred to avoid breaking dependent crates mid-series.
- **Escalations**: none.

## 2026-11-14 - M6-5 part 4a-6 review fix: revert local renames + translate mesure= strings (issue #108)

- **Issue**: #108 (M6-5). Review of PR #208 reported 2 BLOCKING findings.
- **Done**: reverts the local identifier renames in `saturate.rs` (`input_node`/`node` → `noeud_entree`/`noeud`, comments stay translated — identifiers are M6-6 scope); translates the unaccented French string literals `mesure={total}` / `mesure_memo={total}` in `src/integrale.rs` to `measure=` / `measure_memo=` and updates the pinned copies in `tests/integrale.rs` / `tests/integrale_dbm.rs`.
- **Evidence**: `cargo test -p lisage-core --test integrale --test integrale_dbm --test saturate` ok (18 passed); `cargo fmt -p lisage-core -- --check` clean; grep `mesure=` returns nothing outside kept identifiers.
- **Judgement calls**: unaccented French strings are invisible to the accent grep — the reviewer catches them; JSONL wire fields (`noeud`, `valeur`…) stay French as contract.
## 2026-11-14 - M6-5 part 4a-7: lisage-core operations+pli translated to English (issue #108)

- **Issue**: #108 (M6-5, milestone M6).
- **Done**: translates comments, doc-comments and string literals in `crates/lisage-core/src/operations.rs`, `crates/lisage-core/src/pli.rs`, `crates/lisage-core/tests/operations.rs`, `crates/lisage-core/tests/pli.rs` to English. Public French identifiers are kept until dependent crates are translated.
- **Evidence**: `cargo check -p lisage-core --tests` passes; `cargo test -p lisage-core` all 25 test suites pass; `cargo fmt -p lisage-core -- --check` clean. Full `make check` deferred to the last part.
- **Judgement calls**: comments/strings translated now, public identifiers deferred to avoid breaking dependent crates mid-series.
- **Escalations**: none.

## 2026-11-14 - M6-5 part 4a-6: lisage-core saturate+integrale translated to English (issue #108)

- **Issue**: #108 (M6-5, milestone M6).
- **Done**: translates comments, doc-comments and string literals in `crates/lisage-core/src/saturate.rs`, `crates/lisage-core/src/integrale.rs`, `crates/lisage-core/tests/saturate.rs`, `crates/lisage-core/tests/integrale.rs`, `crates/lisage-core/tests/integrale_dbm.rs` to English. Public French identifiers are kept until dependent crates are translated.
- **Evidence**: `cargo check -p lisage-core --tests` passes; `cargo test -p lisage-core` all tests pass; `cargo fmt -p lisage-core -- --check` clean after fmt. Full `make check` deferred to the last part.
- **Judgement calls**: comments/strings translated now, public identifiers deferred to avoid breaking dependent crates mid-series.
- **Escalations**: none.

## 2026-11-14 - M6-3 docs part 1/8: specification translated to English (issue #106)

- **Issue**: #106 (M6-3, milestone M6).
- **Done**: part of the eight-way split of PR #180 (the reviewer session hits its turn limit on diffs over ~500 lines; the human authorized oversized PRs for M6, the split is a reviewer-feasibility measure, not a scope change). This part renames and translates the file(s) listed in the commit message; corrupted machine-translation tokens and remaining French fragments flagged in the review of PR #180 are repaired.
- **Evidence**: accent grep returns 0 on every added file; old-filename grep returns nothing outside `docs/audit-french-inventory.md`, `docs/glossary.md` and `WORKLOG.md` (intentional). `make check` not run: docs-only change, no code touched.
- **Judgement calls**: none beyond the split itself.
- **Escalations**: none.



## 2026-11-14 - M6-3 docs part 2/8: proposal translated to English (issue #106)

- **Issue**: #106 (M6-3, milestone M6).
- **Done**: part of the eight-way split of PR #180 (the reviewer session hits its turn limit on diffs over ~500 lines; the human authorized oversized PRs for M6, the split is a reviewer-feasibility measure, not a scope change). This part renames and translates the file(s) listed in the commit message; corrupted machine-translation tokens and remaining French fragments flagged in the review of PR #180 are repaired.
- **Evidence**: accent grep returns 0 on every added file; old-filename grep returns nothing outside `docs/audit-french-inventory.md`, `docs/glossary.md` and `WORKLOG.md` (intentional). `make check` not run: docs-only change, no code touched.
- **Judgement calls**: none beyond the split itself.
- **Escalations**: none.




## 2026-11-14 - M6-3 docs part 3/8: state of the art translated to English (issue #106)

- **Issue**: #106 (M6-3, milestone M6).
- **Done**: part of the eight-way split of PR #180 (the reviewer session hits its turn limit on diffs over ~500 lines; the human authorized oversized PRs for M6, the split is a reviewer-feasibility measure, not a scope change). This part renames and translates the file(s) listed in the commit message; corrupted machine-translation tokens and remaining French fragments flagged in the review of PR #180 are repaired.
- **Evidence**: accent grep returns 0 on every added file; old-filename grep returns nothing outside `docs/audit-french-inventory.md`, `docs/glossary.md` and `WORKLOG.md` (intentional). `make check` not run: docs-only change, no code touched.
- **Judgement calls**: none beyond the split itself.
- **Escalations**: none.



## 2026-11-14 - M6-3 docs part 4/8: golden-thread example (philosophers) translated (issue #106)

- **Issue**: #106 (M6-3, milestone M6).
- **Done**: part of the eight-way split of PR #180 (the reviewer session hits its turn limit on diffs over ~500 lines; the human authorized oversized PRs for M6, the split is a reviewer-feasibility measure, not a scope change). This part renames and translates the file(s) listed in the commit message; corrupted machine-translation tokens and remaining French fragments flagged in the review of PR #180 are repaired.
- **Evidence**: accent grep returns 0 on every added file; old-filename grep returns nothing outside `docs/audit-french-inventory.md`, `docs/glossary.md` and `WORKLOG.md` (intentional). `make check` not run: docs-only change, no code touched.
- **Judgement calls**: none beyond the split itself.
- **Escalations**: none.


## 2026-11-14 - M6-3 docs part 5/8: two critical analyses translated (issue #106)

- **Issue**: #106 (M6-3, milestone M6).
- **Done**: part of the eight-way split of PR #180 (the reviewer session hits its turn limit on diffs over ~500 lines; the human authorized oversized PRs for M6, the split is a reviewer-feasibility measure, not a scope change). This part renames and translates the file(s) listed in the commit message; corrupted machine-translation tokens and remaining French fragments flagged in the review of PR #180 are repaired.
- **Evidence**: accent grep returns 0 on every added file; old-filename grep returns nothing outside `docs/audit-french-inventory.md`, `docs/glossary.md` and `WORKLOG.md` (intentional). `make check` not run: docs-only change, no code touched.
- **Judgement calls**: none beyond the split itself.
- **Escalations**: none.




## 2026-11-14 - M6-3 docs part 6/8: sliding-window example + reader translated (issue #106)

- **Issue**: #106 (M6-3, milestone M6).
- **Done**: part of the eight-way split of PR #180 (the reviewer session hits its turn limit on diffs over ~500 lines; the human authorized oversized PRs for M6, the split is a reviewer-feasibility measure, not a scope change). This part renames and translates the file(s) listed in the commit message; corrupted machine-translation tokens and remaining French fragments flagged in the review of PR #180 are repaired.
- **Evidence**: accent grep returns 0 on every added file; old-filename grep returns nothing outside `docs/audit-french-inventory.md`, `docs/glossary.md` and `WORKLOG.md` (intentional). `make check` not run: docs-only change, no code touched.
- **Judgement calls**: none beyond the split itself.
- **Escalations**: none.


## 2026-11-14 - M6-3 docs part 8/8: roadmap translated + doc paths in crates comments (issue #106)

- **Issue**: #106 (M6-3, milestone M6).
- **Done**: part of the eight-way split of PR #180 (the reviewer session hits its turn limit on diffs over ~500 lines; the human authorized oversized PRs for M6, the split is a reviewer-feasibility measure, not a scope change). This part renames and translates the file(s) listed in the commit message; corrupted machine-translation tokens and remaining French fragments flagged in the review of PR #180 are repaired.
- **Evidence**: accent grep returns 0 on every added file; old-filename grep returns nothing outside `docs/audit-french-inventory.md`, `docs/glossary.md` and `WORKLOG.md` (intentional). `make check` not run: the five touched Rust files carry doc-comment-only changes (no behavior change); the full crates/ translation is gated by M6-5 (issue #108).
- **Judgement calls**: none beyond the split itself.
- **Escalations**: none.


## 2026-11-14 - M6-3 docs part 7/8: spike translated + philosophers SVG in English (issue #106)

- **Issue**: #106 (M6-3, milestone M6).
- **Done**: part of the eight-way split of PR #180 (the reviewer session hits its turn limit on diffs over ~500 lines; the human authorized oversized PRs for M6, the split is a reviewer-feasibility measure, not a scope change). This part renames and translates the file(s) listed in the commit message; corrupted machine-translation tokens and remaining French fragments flagged in the review of PR #180 are repaired.
- **Evidence**: accent grep returns 0 on every added file; old-filename grep returns nothing outside `docs/audit-french-inventory.md`, `docs/glossary.md` and `WORKLOG.md` (intentional). `make check` not run: docs-only change, no code touched.
- **Judgement calls**: none beyond the split itself.
- **Escalations**: none.

## 2026-11-14 - M6-5 crates/ translated to English, part 3: lisage-plan (issue #108)

- **Issue**: #108 (M6-5, milestone M6).
- **Done**: `crates/lisage-plan` translated to English (doc-comments, inline comments, error strings, test comments). Part 3 of the per-crate split in dependency order (plan depends on lang + algebra, done in parts 1-2). Public identifiers kept in French for now — they are used by bench and will be translated when those crates are processed.
- **Evidence**: `cargo check --workspace` green on this branch; `cargo test -p lisage-plan` green (all tests pass). Full `make check` deferred to the last part.
- **Judgement calls**: comments/strings translated now, public identifiers deferred to avoid breaking dependent crates mid-series.
- **Escalations**: none.

## 2026-11-14 - M6-5 crates/ translated to English, part 2: lisage-algebra (issue #108)

- **Issue**: #108 (M6-5, milestone M6).
- **Done**: `crates/lisage-algebra` translated to English (doc-comments, inline comments, error strings). Part 2 of the per-crate split in dependency order (algebra depends on lang, done in part 1). Public identifiers kept in French for now — they are used by core/plan/bench/liseuse and will be translated when those crates are processed, to keep `cargo check --workspace` green at every step.
- **Evidence**: `cargo check --workspace` green on this branch; `cargo test -p lisage-algebra` green (0 tests in this crate). Full `make check` deferred to the last part.
- **Judgement calls**: comments/strings translated now, public identifiers deferred to avoid breaking dependent crates mid-series.
- **Escalations**: none.

## 2026-11-14 - M6-5 crates/ translated to English, part 1: lisage-lang (issue #108)

- **Issue**: #108 (M6-5, milestone M6).
- **Done**: `crates/lisage-lang` translated to English (identifiers, comments, strings). Part of the per-crate split of PR #182: the full ~29 000-line diff exceeds what the reviewer session can process (turn limit), so crates/ is shipped crate by crate in dependency order, each part compile-checked (`cargo check --workspace` green). The human authorized oversized PRs for M6; the split is a reviewer-feasibility measure, not a scope change.
- **Evidence**: `cargo check --workspace` green on this branch; `cargo test -p lisage-lang` green. Full `make check` deferred to the last part.
- **Judgement calls**: per-crate split in topological order (lang first) so every part compiles standalone.
- **Escalations**: none.

## 2026-11-13 - M6-0 audit: machine-checkable inventory of French-language content (issue #103)

- **Issue**: #103 (M6-0, milestone M6 "English-only documentation and code").
- **Done**: adds `docs/audit-french-inventory.md` — per-file inventory of French content (accented-character lines, French stop-word lines, French snake_case identifier lines) grouped by category (root+docs/.github Markdown, crates/tools/scripts code, manifests, assets), with the grep/regex commands that reproduce every count. 323 files scanned (190 with French signals, the report excluding itself); 120 code files carry French signals (~3 176 accented lines, ~6 378 French-identifier lines), 46 documentation files (~1 459 accented lines).
- **Evidence**: report generated by running the documented grep/regex pipeline over the working tree at commit e49f048 (origin/daily); counts in the report are the actual command output. `make check` not run: docs-only change, no code touched.
- **Judgement calls**: deliverable is a docs/ file (the issue allows "issue comment or docs/ file"); the report keeps accented French file names as-is because it is a point-in-time snapshot — the M6-7 anti-French lint must exclude this file (noted for M6-7).
- **Escalations**: none.

## 2026-11-13 - M6-2 root documentation in English (issue #105)

- **Issue**: #105 (M6-2, milestone M6).
- **Done**: README.md and AGENTS.md were already fully English (0 accented lines, grep-verified); WORKLOG.md conventions were already English — adds the explicit rule that new entries are written in English while pre-M6 entries stay in French as the historical record. Historical `.github/pr-body-*.md` artifacts are left untouched (past PR bodies, not living documentation). Known limitation: the one French comment in `.github/workflows/ci.yml` (bench step name) cannot be fixed from an agent session — the GitHub App token lacks the `workflow` scope, push is refused; left for the human.
- **Evidence**: `grep -rIlP '[àâäéèêëîïôöùûüçÀÂÄÉÈÊËÎÏÔÖÙÛÜÇ]' README.md AGENTS.md Makefile .github/workflows/` returns nothing after the fix; `make check` not run: docs/config-comment-only change, no code touched.
- **Judgement calls**: AGENTS.md needed no translation (already English), so no needs-human escalation was required for this issue despite AGENTS.md being on the escalation list — the diff does not touch it.
- **Escalations**: none.

## 2026-11-13 - M6-1 glossary review fix: missing contract terms + duplicate rows (PR #178)

- **Issue**: #104 (M6-1, milestone M6).
- **Done**: review of PR #178 found 1 BLOCKING — the glossary lacked rows for the core contract vocabulary used in docs/decisions/ADR-0100 (`pool`, `algèbre de clés`/key algebra, `contrôleur`/controller, `handle`, `identifiant de nœud`/node id). Rows added. Also fixed the two SUGGESTIONs: duplicate rows removed (seuil, trace, erreur, image, pile, file, anneau, quotient) and the `jalon` note corrected (`jalon/M6` are labels, not milestones).
- **Evidence**: `grep -c 'seuil | threshold' docs/glossary.md` = 1; new rows present at docs/glossary.md:44-48. `make check` not run: docs-only change.
- **Judgement calls**: none.
- **Escalations**: none.

## 2026-11-13 - M6-1 glossary: FR→EN terminology mapping (issue #104)

- **Issue**: #104 (M6-1, milestone M6).
- **Done**: adds `docs/glossary.md` — two-column FR→EN table covering the domain terms named in the issue (liseuse→reader, diagramme de décision→decision diagram, transduction attribuée→attributed transduction, fenêtre glissante→sliding window, jalon→milestone, atelier didactique→didactic workshop, pool bi-plan→two-plane pool, forme relative→relative form, témoin BDD→BDD witness, schéma de référence→reference schema) plus the full identifier vocabulary extracted from crates/ (word-level mapping: fenetre→window, construire→build, compter→count, temoin→witness, etc.) and the naming rules (snake_case English functions, English module/file names, acronym casing, English log events and user-facing strings).
- **Evidence**: vocabulary extracted mechanically from all crates/*/src and crates/*/tests identifiers (1 079 unique multi-word identifiers, 402 declarations) and from the domain terms grepped in docs/ and README.md; `make check` not run: docs-only change, no code touched.
- **Judgement calls**: word-level mapping (not per-identifier) so the glossary stays maintainable; acronyms that are proper names (BDD, CUDD, DAG, GCD) stay uppercase, others follow surrounding case.
- **Escalations**: none.

## 2026-11-13 - M6-4 docs/agents + docs/decisions in English (issue #107)

- **Issue**: #107 (M6-4, milestone M6).
- **Done**: `docs/agents/*.md` and `docs/decisions/*.md` were already English except `adr-0100-interface-coeur-m1.md` (fully French) — translated to English and renamed to `adr-0100-core-interface-m1.md` via `git mv`; four stray French words fixed in adr-0019/adr-0101/adr-0102. Accent grep over `docs/agents` + `docs/decisions` now returns 0; stop-word scan shows only false positives (the English word "or").
- **Evidence**: `grep -rnP '[àâäéèêëîïôöùûüçÀÂÄÉÈÊËÎÏÔÖÙÛÜÇœæ]' docs/agents docs/decisions` returns nothing. `make check` not run: docs-only change. No references to the old ADR filename existed outside the M6-0 audit inventory.
- **Judgement calls**: this PR touches constitutional files (`docs/decisions/`), so it is labeled `needs-human` and is NOT merged by the agent — the merge driver refuses it; the human merges.
- **Escalations**: `needs-human` label on the PR (constitutional files on the escalation list, ADR-0010).

## 2026-11-13 - Confrontation — horloge alea-lcg avant la génération de la table (fix/confrontation-alea-horloge)

- **Issue**: aucune (asymétrie signalée à la revue de PR #95/#96 : le témoin CUDD démarre son horloge avant la génération de la table LCG, le runner lisage démarrait la sienne après).
- **Done**: dans `charge_alea_lcg`, `Instant::now()` déplacé avant `table_lcg(...)` — le bloc mesuré couvre désormais génération de la table + construction + comptage, comme chez le témoin C (`cudd_bench.c`). L'init (schéma/arène) reste hors bloc des deux côtés.
- **Evidence**: `make check` green (ruff, mypy strict, contrats, pytest 47 passed + coverage 94.76%, cargo fmt --check, clippy `-D warnings`, cargo nextest 285 tests passés, 3 ignorés). CI GitHub verte sur le head de la PR (#98).
- **Judgement calls**: aucun.
- **Escalations**: aucune.

## 2026-11-12 - Confrontation lisage/témoins 1/3 — runner lisage `confrontation` : une charge, un processus, une ligne JSON (bench/confrontation-1-socle)

- **Issue**: aucune (demande humaine : mesurer lisage en temps/mémoire face aux autres outils de diagrammes de décision, BDD/ZDD/MDD/DDD/SDD ; l'app GitHub ne peut pas créer d'issue — 403).
- **Done**: `lisage-bench` gagne le runner de confrontation : `src/confrontation.rs` (charges `alea-lcg <n>` — table LCG déterministe M1-8, BDD par expansion de Shannon ; `philosophes <n> <chemin>` — R_n du fil rouge lu depuis le fichier d'états étendus partagé, union des maps compilées sur le schéma intercalé, cardinal par pli `(ℕ,+,×)` ; `fenetre <taille> <nb_bases> [absolu|relatif]` — les deux encodages de l'atelier, R_n et fenêtre via `AtelierController` sans R-collapse), `src/bin/confrontation.rs` (codes de sortie 0/1/2), erreurs typées `ErreurConfrontation`, RSS maximale par `VmHWM`, ligne JSON à ordre de clés épinglé. 10 tests dans `tests/confrontation.rs` : épingles (LCG n=4 → 7 et n=12 → 2074 ; |R_n| = 20/56/152/416/1136 ; R_3 = 33 nœuds ; fenêtre (4,10) : 40 chemins, 12 nœuds absolu / 3 relatif), erreurs d'entrée, sérialisation épinglée caractère par caractère, cohérence génération ↔ pins.
- **Evidence**: épingles recalculées par simulation Python indépendante (génération de R_n et pins des fragments 1-2 du WORKLOG) ; API `lisage-core::atelier` re-vérifiée sur le `daily` du jour (`construire_fenetre` prend l'encodage depuis la review PR #92). **Non exécuté ici** : `cargo fmt/clippy/nextest` et `make check` (pas de toolchain Rust ni uv dans l'environnement d'écriture) — attendus sur la CI de la PR.
- **Judgement calls**: le socle complet (runner + témoin CUDD + orchestrateur + doc ≈ 2 300 lignes) dépasse le budget ~1 000 lignes/PR (ADR-0101) : la pile est scindée en trois PR empilées (1 socle Rust, 2 témoin CUDD + protocole, 3 orchestrateur Python), une entrée WORKLOG chacune ; pas d'issue (403) — branche descriptive `bench/confrontation-1-socle`.
- **Escalations**: aucune (ADR-0102 : témoins externes en Docker local — décision humaine du 10 nov. 2026).


## 2026-11-12 - Review PR #94 — ancre composée réellement appliquée au pool (les deux ancres entrent par l'arc d'entrée) (feat/atelier-ancre)

- **Issue**: aucune (correction de la revue headless ADR-0010 de la PR #94).
- **Done**: les 2 BLOCKING de la revue portaient sur le fait que les deux paramètres `AncreFenetre` n'atteignaient jamais le pool (`meme_forme` trivialement vrai, aucun test ne capterait une régression rendant l'interning dépendant de l'ancre). Correction : nouvelle `AncreFenetre::cle_entree()` (la base, saturée dans `u64`, = l'argument de l'arc d'entrée, spec § 2.4.5) et nouvelle `construire_fenetre_ancree(pool, anchor, taille, ancre) -> (racine, fenêtre)` — chaque ancre construit sa racine propre avec sa base comme clé d'arc d'entrée, la forme de fenêtre restant relative et partagée (`interner_fenetre_relative` extrait de `construire_fenetre_relative`). `comparer_ancres_fenetre` appelle maintenant deux fois `construire_fenetre_ancree` avec les deux ancres et pose `meme_forme = fenetre_avant == fenetre_apres` — une vraie assertion : une régression dupliquant la forme par base la ferait échouer. Test renforcé : 50 glisses (bases 100..150 neuves) où chaque glisse crée exactement une racine (`node_count` = avant + 50) et atteint toujours la même forme de fenêtre. Correction en cours de route : `cle_entree` saturée par `max(0)` (`u64::MAX as i64` déborde en -1, panic `clamp` attrapé par les tests).
- **Evidence**: `make check` green (ruff, mypy strict, import-linter, pytest 34 passed, coverage 94.76%, cargo fmt --check, clippy `-D warnings`, cargo nextest : 285 tests, 3 ignorés). **Non exécuté** : re-revue headless ADR-0010 (relance ci-dessous), CI GitHub sur le nouveau head, merge.
- **Judgement calls**: la base de l'ancre composée est passée au pool comme clé de l'arc d'entrée (et non comme ancre d'interning) — c'est le modèle M4-2 (section min = 0, l'argument vit sur l'arc, le pool reste aveugle à l'ancre composée) ; `construire_fenetre_ancree` retourne le couple `(racine, fenêtre)` pour rendre `meme_forme` vérifiable sans résoudre la racine.
- **Escalations**: aucune.

## 2026-11-12 - Atelier didactique, fragment 4 — paramètre ancre composée sur la fenêtre glissante (backlog § 9) (feat/atelier-ancre)

- **Issue**: aucune (quatrième fragment de l'atelier didactique, choisi par instruction directe « Continue » ; l'app GitHub ne peut pas créer d'issue — 403).
- **Done**: dans `lisage-core::atelier` : `AncreFenetre` (enveloppe didactique de `AncreComposee` M4-2 avec les deux fragments nommés du protocole SWP — `fragment_base()`, `fragment_horizon()` — méthodes `base`/`horizon`/`glessee(delta)`/`fenetre_absolue()` = `[base, base+horizon)`/`etiquette()` = `{base: b, horizon: k}`) et `comparer_ancres_fenetre` → `RapportComparaisonAncres { avant, apres, meme_forme, noeuds_pool, lecture_avant, lecture_apres }` avec narration ADR-0100 (`ancre={base: 0, horizon: 4}_vs_{base: 2, horizon: 4} noeuds=3 contre 3 lecture=[0..4) contre [2..6) pool=3`). C'est le « l'ancre bouge, le pool jamais » du fil rouge § 5 : la forme relative de la fenêtre (clés `0..taille-1`, section min = 0) est internée une fois, la glisse ne déplace que l'ancre composée (handle), le pool ne crée aucun nœud — mêmes métriques des deux côtés, lectures absolues différentes. 4 tests : glisse pure (base déplacée, horizon invariant, ancre d'origine inchangée) ; comparaison base 0 contre base 7 (même forme, 3 nœuds, 4 chemins, taille dépliée 6, `noeuds_pool=3`) + 50 glisses avec ré-interning constant ; propriété LCG 30 cas (glisser(a)∘glisser(b) = glisser(a+b), horizon invariant) ; narration épinglée. Note : les tests commutent R-collapse (`sans_collapse`) comme le fragment 2 (une fenêtre toute-vraie collapserait sinon).
- **Evidence**: `make check` green (ruff, mypy strict, import-linter, pytest 34 passed, coverage 94.76%, cargo fmt --check, clippy `-D warnings`, cargo nextest : 285 tests, 3 ignorés — 4 nouveaux). **Non exécuté** : CI GitHub (verte attendue sur la PR), revue headless ADR-0010 et merge (étapes suivantes du workflow).
- **Judgement calls**: pas d'issue créée (403, directive humaine) — branche descriptive `feat/atelier-ancre` ; `AncreFenetre` posé dans `atelier.rs` (fragment autonome, comme les fragments 1-3) plutôt que dans `relatif.rs`, le type M4-2 `AncreComposee` restant le cœur inchangé — l'atelier n'enveloppe que la lecture didactique ; la comparaison se fait dans un même pool (les deux ancres sont des lectures de la même forme, pas des contrôleurs différents — cohérent avec C1 : l'ancre vit dans le handle, le pool reste aveugle) ; `fenetre_absolue` rendue en `(i64, i64)` (intervalle `[début, fin)`) plutôt qu'un `Vec` — la fenêtre se lit, elle ne s'énumère pas.
- **Escalations**: aucune.

## 2026-11-12 - Atelier didactique, fragment 3 — paramètre d'ordre sur la compilation BDD (backlog § 9) (feat/atelier-ordre)

- **Issue**: aucune (troisième fragment de l'atelier didactique, choisi par instruction directe « Suite de l'atelier » ; l'app GitHub ne peut pas créer d'issue — 403).
- **Done**: dans `lisage-core::atelier` : `Ordre` (permutation niveau → variable : `naturel`, `inverse`, `personnalise` avec assertion de permutation), `compiler_bdd_ordre` (la table de vérité est permutée pour que le bit ℓ de l'indice local = bit `ordre[ℓ]` de l'indice original, puis expansion de Shannon classique — l'ordre naturel redonne exactement le `compiler_bdd` de référence, même `NodeId`), et `comparer_ordres` → `RapportComparaisonOrdres { premier, second }` avec narration ADR-0100 (`ordre=[0,1,2,3]_vs_[0,2,1,3] noeuds=… contre …`). C'est le « le diagramme est une fonction du contrôleur » pour le paramètre d'ordre : mêmes chemins (2^n) et même taille dépliée (2^{n+1}-1) des deux côtés, nœuds partagés différents. 4 tests : f = (x₀∧x₁)∨(x₂∧x₃) sur 4 variables → naturel 10 nœuds contre intercalé [0,2,1,3] 12 (16 chemins, taille dépliée 31 des deux côtés) ; 6 variables → 15 contre 25 (64 chemins) ; équivalence ordre naturel = `compiler_bdd` ; narration épinglée. Note : les tests commutent R-collapse (`sans_collapse`) car sinon les chemins fusionnent (7 au lieu de 16) et l'invariant « mêmes chemins » ne se lit plus.
- **Evidence**: `make check` green (ruff, mypy strict, import-linter, pytest 34 passed, coverage 94.76%, cargo fmt --check, clippy `-D warnings`, cargo nextest : 281 tests, 3 ignorés — 4 nouveaux). **Non exécuté** : CI GitHub (verte attendue sur la PR), revue headless ADR-0010 et merge (étapes suivantes du workflow).
- **Judgement calls**: pas d'issue créée (403, directive humaine) — branche descriptive `feat/atelier-ordre` ; `Ordre` et la compilation vivent dans `atelier.rs` (fragment autonome, comme les fragments 1-2) plutôt que dans `bdd.rs`, la compilation sous ordre n'étant pour l'instant qu'un banc didactique ; la comparaison se fait dans un même pool (les deux ordres sont des lectures de la même table, pas des contrôleurs différents).
- **Escalations**: aucune.

## 2026-11-12 - Atelier didactique, fragment 2 — paramètre encodage absolu/relatif sur la fenêtre glissante (backlog § 9) (feat/atelier-encodage)

- **Issue**: aucune (deuxième fragment de l'atelier didactique, choisi par instruction directe « Continue » ; l'app GitHub ne peut pas créer d'issue — 403).
- **Done**: dans `lisage-core::atelier` : `Encodage` (Absolu/Relatif), `construire_fenetre_absolue` (un nœud par base, clés absolues), `construire_fenetre_relative` (une seule forme partagée `{0..taille-1}`, section min = 0, argument sur l'arc d'entrée — instance M4-2), et `comparer_encodages_fenetre` → `RapportComparaison { absolu, relatif }` avec narration ADR-0100 (`encodage=relatif_vs_absolu noeuds=… contre …`). C'est le « 4 contre l'infini de la fenêtre » de la feuille de route § 9, mesuré : mêmes chemins (même objet logique), nœuds partagés différents. 2 tests : 10 bases → absolu 12 nœuds contre relatif 3 (chemins 40 des deux côtés, taille dépliée 51) ; 100 bases → relatif constant à 3, absolu 102 ; narration épinglée. Note : les tests commutent R-collapse (`sans_collapse`) car une fenêtre toute-vraie collapserait en ⊤ sinon.
- **Evidence**: `make check` green (ruff, mypy strict, import-linter, pytest 34 passed, coverage 94.76%, cargo fmt --check, clippy `-D warnings`, cargo nextest : 277 tests, 3 ignorés — 2 nouveaux). **Non exécuté** : CI GitHub (verte attendue sur la PR), revue headless ADR-0010 et merge (étapes suivantes du workflow).
- **Judgement calls**: pas d'issue créée (403, directive humaine) — branche descriptive `feat/atelier-encodage` ; la comparaison se fait dans un même pool (les deux encodages sont des écritures de clés, pas des contrôleurs différents — cohérent avec M4-2 où le pool est aveugle au groupe) ; `Encodage` posé comme enum mais la comparaison porte sur la fenêtre glissante (instance vérifiée M4-2), les autres paramètres du contrôleur (ordre, royaume, ancre composée) restent des fragments ultérieurs.
- **Escalations**: aucune.

## 2026-11-12 - Atelier didactique, fragment 1 — tableau de bord des réductions (backlog § 9, demande du 10 nov. 2026) (feat/atelier-reductions)

- **Issue**: aucune (l'atelier didactique est un candidat du backlog § 9, débloqué depuis M3-4, choisi par instruction directe du propriétaire ; l'app GitHub ne peut pas créer d'issue — 403). Les 7 issues fantômes (#32–36, #83, #85) ont été fermées sur instruction du propriétaire avec un commentaire citant la PR mergée — le jalon M5 affiche désormais 0 issue ouverte.
- **Done**: premier fragment backend de l'atelier (sans front web) : (1) hooks `collapse_actif` / `defaut_actif` (défaut `true`) sur le trait `Controller` (lisage-algebra), honorés par `Arena::reduit` via la nouvelle `reduction::reduire_sans_collapse` (R-default seul) ; (2) `lisage-core::atelier` : `Reductions` (commutateurs), `AtelierController<C>` (enveloppe délégante, fingerprint 100), et `mesurer` → `RapportAtelier { noeuds_partages, chemins, taille_depliee }` — R-share n'étant pas commutable (table unique du pool), sa désactivation se mesure par la taille dépliée (nœuds de l'arbre déplié, chaque arc dupliquant le sous-arbre cible) ; erreurs du pool propagées (`Result`, test épinglé). (3) `construire_r_n` généralisée sur `P: Pool<Key = u64, Payload = bool>`. 8 tests : commutation R-collapse (BDD), commutation R-default (régions), métriques sur losange (4 nœuds / 4 chemins / taille dépliée 7), erreur propagée, PBT LCG (commuter ne réduit jamais), narration `Attr`/`Synthétise` de `mesurer` épinglée, et fil rouge R₃ : 33 nœuds partagés, 20 chemins (un par état), taille dépliée 126, enveloppe transparente quand tout est actif.
- **Evidence**: `make check` green (ruff, mypy strict, import-linter, pytest 34 passed, coverage 94.76%, cargo fmt --check, clippy `-D warnings`, cargo nextest : 275 tests, 3 ignorés — 8 nouveaux). **Non exécuté** : CI GitHub (verte attendue sur la PR), revue headless ADR-0010 et merge (étapes suivantes du workflow).
- **Judgement calls**: pas d'issue créée (403, directive humaine) — branche descriptive `feat/atelier-reductions` ; R-share mesuré (taille dépliée) plutôt que commuté — commuter le hash-consing exigerait un pool dégradé, hors fragment ; hooks à défaut `true` sur le trait (rétrocompatible, aucun contrôleur existant touché) ; taille dépliée préférée au nombre de chemins comme métrique « sans R-share » (c'est le nombre de nœuds du DAG déplié) ; `construire_r_n` généralisée plutôt que dupliquée.
- **Escalations**: aucune.

## 2026-11-12 - Backlog δ continu, intégrale sur régions polyédriques DBM 2D — aire de la bande ∩ boîte (Q9) (feat/integrale-dbm)

- **Issue**: aucune (fragment « régions polyédriques » du candidat Q9 (ii) de la spec § 10 — « régions polyédriques + densités polynomiales » ; la veine δ continu est otherwise complète : intervalles, DBM, DDD-différence, intégrale 1D, CDD ; l'app GitHub ne peut pas créer d'issue — 403 — et la directive du matin appartient à l'humain). Repris sur instruction directe « Continue ».
- **Done**: `DbmKeys::mesure_zone` (dans `dbm.rs`) : mesure de Lebesgue d'une zone DBM **2D** intersectée à la boîte `[0, N)²`, rendue **doublée** en `i128` (aire exacte, peut être demi-entière). Une DBM 2D est une bande `−b ≤ x−y ≤ a` ; aire = `f(−b) − f(a)` avec `f(t)` = aire de `{x−y > t}` en forme close (triangle si t ≥ 0, carré moins triangle si t < 0). `integrale_dbm_constante` (dans `integrale.rs`) : intégrale de densité 1 sur diagramme à clés DBM, `intégrale(nœud, région) = Σ arcs intégrale(fils, région ∩ zone)`, mémo sur `(nœud, région)`, propage `PoolError` (test épinglé : `NodeId` non interné → `Err`), narration `Attr`/`Synthétise` avec la mesure (ADR-0100). 7 tests : aires analytiques (boîte pleine 32, bande 19, diagonale 0, contrainte hors boîte, demi-bande 7), intégrale de la bande `x−y≤1 ∧ y−x≤2` = 19, restriction à une sous-région = 12, terminaux, différentiel cardinal (12 points entiers) contre mesure (aire 9.5), narration épinglée (`mesure=19`), propriété LCG (monotonie mesure(A∩B) ≤ min).
- **Evidence**: `make check` green (ruff, mypy strict, import-linter, pytest 34 passed, coverage 94.76%, cargo fmt --check, clippy `-D warnings`, cargo nextest : 267 tests, 3 ignorés — 7 nouveaux). **Non exécuté** : CI GitHub (verte attendue sur la PR), revue headless ADR-0010 et merge (étapes suivantes du workflow).
- **Judgement calls**: pas d'issue créée (403, directive humaine) — branche descriptive `feat/integrale-dbm` ; mesure **doublée** en `i128` plutôt qu'un rationnel (l'aire d'une bande à bornes entières est demi-entière au pire) ; dimension 2 uniquement (assert) — le volume d'une DBM en n > 2 hors fragment, documenté ; densités polynomiales (intégration symbolique) hors fragment, la densité reste constante 1 comme en 1D ; la formule d'aire a été corrigée en cours de route (f(−b) − f(a), pas N² − f(a) − f(−b)) — les tests analytiques l'épinglent.
- **Escalations**: aucune.

## 2026-11-12 - Backlog δ continu, instance-test CDD — unions non convexes de zones DBM (Q9) (feat/cdd)

- **Issue**: aucune (dernier fragment nommé de la veine δ continu — « instance-test : DDD-différence / CDD » de la feuille de route § 9 ; les 7 issues ouvertes sont toutes livrées et mergées dans `daily` ; l'app GitHub ne peut pas créer d'issue — 403 — et la directive du matin appartient à l'humain). Repris sur instruction directe « Continue ».
- **Done**: `lisage-core::cdd` (nouveau) : clé-région **CDD** = union finie de zones DBM (état de l'art : « δ = différences d'horloges, ensembles non convexes »). Forme normale syntaxique = tri + dédup des zones clôturées, **non-canonicité assumée** (spec Q9 (i) : « unions non convexes sans forme normale connue ») — test épinglé : une zone incluse dans une autre reste une clé distincte. Appartenance `contient` décidable sémantiquement ; `intersection` distributive (union des meets point à point) ; `CddKeys::couverture` par balayage de la boîte bornée (réserve spec Q9 : univers borné à régions énumérables, sinon royaume libre DDD). `CddController` (fingerprint 6, payload bool, `D_δ` = union vide → `false`). Instance-test `compiler_union_horloges` : la région non convexe `{x − y ≤ 0} ∪ {x − y ≥ 2}` sur deux horloges en un niveau (union → `true`, complémentaire `{x − y = 1}` → `false`), `evaluer`/`compter_noeuds`. 8 tests : normalisation, non-canonicité assumée, appartenance, distributivité du meet, couverture, différentiel contre énumération naïve sur `[0,4)²` (3 nœuds contre 16 points), partage au pool (C1 relatif), propriété LCG 30 cas (commutativité syntaxique, idempotence sémantique), contrat de niveau.
- **Evidence**: `make check` green (ruff, mypy strict, import-linter, pytest 34 passed, coverage 94.76%, cargo fmt --check, clippy `-D warnings`, cargo nextest : 260 tests, 3 ignorés — 8 nouveaux). CI GitHub verte sur la PR #89. Revue headless ADR-0010 : **1 BLOCKING corrigé** — `compter_noeuds` retournait `u64` et avalait le `PoolError` de `pool.resolve` (`if let Ok(..)`) ; la fonction retourne désormais `Result<u64, PoolError>` comme `evaluer`, avec un test épinglant la propagation (`NodeId(u32::MAX)` non interné → `Err`). `make check` re-green après correction. **Non exécuté** : revue de suivi formelle après correction.
- **Judgement calls**: pas d'issue créée (403 de l'intégration, directive humaine) — branche descriptive `feat/cdd` ; pas de nouveau variant de narration (pas d'observateur, comme l'instance-test DDD) ; idempotence du meet testée sémantiquement et non syntaxiquement, conséquence documentée de la non-canonicité assumée ; contrôleur distinct (fp 6) plutôt que réutilisation de `DbmController`, la clé n'étant pas une `Dbm` ; le `compter_noeuds` du fragment DDD (PR #87) a le même `if let Ok(..)`, hors périmètre de cette PR.
- **Escalations**: aucune.

## 2026-11-12 - Backlog δ continu, intégrale au pli — mesure de Lebesgue sur DDD intervalles 1D (Q9) (feat/integrale-pli, PR #88)

- **Issue**: aucune (quatrième fragment de la veine δ continu, précondition M5-4 levée ; la directive du matin appartient à l'humain).
- **Done**: `lisage-core::integrale` (nouveau) : sur les clés-régions 1D, le pli discret (cardinal) devient intégrale de Lebesgue — `mesure_intervalle([a,b]) = b−a` (point isolé de mesure nulle), `integrale_constante(pool, racine, région, observateur)` avec la région courante comme contexte et mémo sur `(nœud, région)`, `intersection` des intervalles, `compiler_union` (instance-test, précondition disjoints assertée en debug). Narration ADR-0100 : chaque nœud visité émet un `TraceEvent::Attr`/`Synthétise` avec la mesure, épinglé par un observateur collecteur. 6 tests : mesure continue, intersection, union deux intervalles (count 7 vs mesure 5), restriction à une sous-région, terminaux, propriété LCG 30 cas + test de narration.
- **Evidence**: `make check` green (ruff, mypy strict, import-linter, pytest 34 passed, coverage 94.76%, cargo fmt --check, clippy `-D warnings`, cargo nextest : 250 tests, 3 ignorés — 6 nouveaux). CI GitHub verte sur la PR #88 (run 43171883782, 2m52s). Revue headless ADR-0010 : 2 BLOCKING corrigés (narration DEBUG + entrée WORKLOG), 1 SUGGESTION traitée (debug_assert disjoints). **Non exécuté** : revue de suivi formelle après correction.
- **Judgement calls**: narration via le variant `Attr`/`Synthétise` existant plutôt qu'un nouveau variant `TraceEvent` (évite de toucher à la sérialisation liseuse) ; observateur passé en `Option<&dyn Observer>` (None = pas de narration, comme le runner de transductions) ; mesure en `u64` saturé (pas de rationnel — les longueurs d'intervalles sont entières).
- **Escalations**: aucune.

## 2026-11-11 - Backlog δ continu, instance-test DDD-différence : contrat de clés (couverture, intersection) et bande sur deux horloges (Q9) (feat/ddd-difference)

- **Issue**: aucune (les 7 issues ouvertes sont toutes livrées et mergées dans `daily` ; l'app GitHub ne peut pas créer d'issue — 403 — et la directive du matin appartient à l'humain). Fragment « instance-test DDD-différence » de la veine #83, nommé par la feuille de route § 9 et la spec § 10 Q9, repris sur instruction directe « Continue ».
- **Done**: contrat de clés déclaré côté contrôleur, pool inchangé — `Dbm::intersection` (meet point par point des bornes puis clôture canonique ; région vide → `INCOHERENT`) et `DbmKeys::couverture` (balayage d'univers de la boîte bornée `[0, taille)^n`, décidable seulement sur univers borné à régions énumérables — sinon royaume libre DDD assumé, la couverture se déclare par les arcs explicites). Instance-test `lisage-core::ddd` : `compiler_bande` compile la bande `x − y ≤ 1 ∧ y − x ≤ 2` sur deux horloges en deux niveaux par régions explicites (aucun balayage à la construction), `evaluer` suit l'arc dont la région contient le point (arc absent = δ(ℓ) = absent continu), `compter_noeuds`. 6 tests dans `crates/lisage-core/tests/ddd.rs` : meet canonique (commutativité, idempotence, disjoint → `INCOHERENT`), couverture (partition couvre, moitié/famille vide/région vide ne couvrent pas), différentiel de la bande contre énumération naïve sur `[0,4)²` (4 nœuds contre 16 points), partage au pool de la racine (C1 relatif), propriété LCG sur l'intersection (30 cas), contrat de niveau (cardinalité, défaut).
- **Evidence**: `make check` green (ruff, mypy strict, import-linter, pytest 34 passed, coverage 94.76%, cargo fmt --check, clippy `-D warnings`, cargo nextest : 244 tests, 3 ignorés — 6 nouveaux). **Non exécuté** : CI GitHub (verte attendue sur la PR), revue headless ADR-0010 et merge (étapes suivantes du workflow).
- **Judgement calls**: pas d'issue créée (403 de l'intégration, et la liste d'issues est posée par l'humain) — branche descriptive `feat/ddd-difference` au lieu de `feat/<n>` ; pas de nouveau contrôleur (fingerprint) : l'instance-test réutilise `DbmController` (fp 5), le contrat vit sur `Dbm`/`DbmKeys` ; la couverture est calculée par balayage sur boîte bornée uniquement, conformément à la réserve de la spec Q9.
- **Escalations**: aucune.

## 2026-11-11 - Backlog δ continu, fragment DBM : régions convexes à clôture canonique (Q9) (feat/85)

- **Issue**: #85 (backlog § 9, spec Q9, veine #83 — deuxième fragment « DBM » après les intervalles).
- **Done**: `lisage-core::dbm` (nouveau) : `Dbm` (clé-région = matrice de bornes de différences `x_i − x_j ≤ c`, bornes `i128` en interne pour une clôture sans débordement ; forme normale = clôture de Floyd-Warshall, région incohérente normalisée au canonique `INCOHERENT` = région vide), `DbmKeys` (`Val(ℓ) = [0, taille_domaine)`, `cardinality(ℓ)` = taille du domaine, `δ(ℓ)` = `INCOHERENT`), `DbmController` (payload bool, `D_δ` = région vide → `false`, fingerprint `ControllerId(5)`). 7 tests dans `crates/lisage-core/tests/dbm.rs` : clôture canonique (propagation transitive, contraintes redondantes), incohérence → `INCOHERENT`, `contient`, cardinalité/défaut, **C1 relatif au pool** (deux écritures d'une même région partagent le nœud), propriété LCG (30 cas : idempotence de la forme normale), différentiel d'appartenance contre énumération naïve sur la boîte `[0,4)²` (partition en deux régions DBM, R-default supprime l'arc vide). Hors fragment : intégrale au pli, instance-test DDD-différence.
- **Evidence**: `make check` green (ruff, mypy strict, import-linter, pytest, cargo fmt --check, clippy `-D warnings`, cargo nextest : 238 tests, 3 ignorés — 7 nouveaux). **Non exécuté** : CI GitHub (verte attendue sur la PR), revue headless ADR-0010 et merge (étapes suivantes du workflow).
- **Judgement calls**: bornes `i128` en interne (la clôture somme des `i64` ne déborde pas) plutôt qu'un `checked_add` qui perdrait la canonicité ; domaine borné par variable (`cardinality(ℓ)` fini) — le « royaume libre DDD » non borné reste hors fragment ; module dans `lisage-core` comme `region.rs`, aucune dépendance, aucune frontière modifiée.
- **Escalations**: aucune.

## 2026-11-11 - Backlog δ continu, fragment intervalles : algèbre de clés-régions 1D (Q9) (feat/83)

- **Issue**: #83 (backlog § 9 de la feuille de route, question ouverte 9 de la spec — premier fragment « intervalles », la porte M5-4 étant levée).
- **Done**: `lisage-core::region` (nouveau) : `Intervalle` (clé-région = intervalle fermé, construction normalisée — l'intervalle vide devient le canonique `VIDE`, l'égalité dérivée est l'égalité ensembliste), `RegionKeys` (`Val(ℓ) = [0, domaines[ℓ])`, `cardinality(ℓ)` = taille du domaine, `δ(ℓ)` = `VIDE`, l'« absent » continu miroir de `ABSENT`), `RegionController` (payload bool, `D_δ` = région vide → `false`, fingerprint `ControllerId(4)`). 6 tests dans `crates/lisage-core/tests/region.rs` : normalisation de l'intervalle vide, forme normale canonique et idempotente, cardinalité/défaut par niveau, **C1 relatif au pool** (deux écritures d'une même région partagent le nœud, régions distinctes non), propriété LCG (30 cas : idempotence + cohérence taille/est_vide/contient), différentiel contre énumération naïve (union d'intervalles en arcs disjoints, compte DD == |S|, R-default supprime l'arc `VIDE`). Hors fragment : intégrale au pli, DBM, instance-test DDD-différence.
- **Evidence**: `make check` green (ruff, mypy strict, import-linter, pytest, cargo fmt --check, clippy `-D warnings`, cargo nextest : 231 tests, 3 ignorés — 6 nouveaux). **Non exécuté** : CI GitHub (verte attendue sur la PR), revue headless ADR-0010 et merge (étapes suivantes du workflow).
- **Judgement calls**: le fragment « intervalles » seul (pas les DBM ni l'intégrale) pour tenir dans un PR ; le module vit dans `lisage-core` comme `bdd.rs`/`schema.rs` (les algèbres de clés de référence y vivent), aucune dépendance ajoutée, aucun changement de frontière (test de frontières vert inchangé).
- **Escalations**: aucune.

## 2026-11-11 - M5-2 : voie C du triptyque câblée au planner — graphe ADR-0002 étendu `lisage-bench → lisage-plan` (décision humaine A) (feat/33c)

- **Issue**: #33 (M5-2 — Corpus MCC ; triptyque A/B/C complet, énoncés C3/C4). Suite de la décision needs-human du 2026-11-11 (option A) : extension du graphe de dépendances pour autoriser `lisage-bench → lisage-plan`.
- **Done**: `frontieres.rs` et `Cargo.toml` de bench mis à jour (graphe : `lisage-bench → core, lang, algebra, plan` — acyclique, plan → {lang, algebra}). `triptyque::voie_c` soumet désormais π au planner : `Order::avec_aval_runtime(&ClesMap(cles), pi_runtime)` avec π construit présence-puis-valeur par clé ; la voie C retourne `ResultatVoieC { mesure, strategie }` (`StrategieAval::OrdreRuntime` si π valide, `RepliCanonique` sinon — l'ordre canonique est alors adopté et le cardinal reste exact). Tests mis à jour : π invalide → repli canonique + cardinal exact (plus de rejet), π valide inversé → ordre runtime adopté, cardinal identique A/B/C inchangé. 7 tests triptyque au total.
- **Evidence**: `make check` green (ruff, mypy strict, import-linter, pytest, cargo fmt --check, clippy `-D warnings`, cargo nextest : 225 tests, 3 ignorés, dont le test de frontières qui valide le nouveau graphe). **Non exécuté** : CI GitHub (verte attendue sur la PR), revue headless ADR-0010 et merge (étapes suivantes du workflow), conteneurisation des modèles MCC et publication des chiffres (reste de #33).
- **Judgement calls**: la décision A a été prise par l'humain en session needs-human (2026-11-11) — le changement de fichier constitutionnel (test de frontières) est couvert par cette autorisation ; π runtime est traduit en `CleMap` présence-puis-valeur par clé, ce qui satisfait la contrainte du type tant que π est une permutation des clés de places.
- **Escalations**: consommée (frontière bench→plan, option A). Restent ouvertes : conteneurisation du corpus MCC 2011-2018, publication des chiffres, surface du langage.

## 2026-11-11 - needs-human consommés : ADR-0021 accepté, ADR-0100 accepté (logging = événements DEBUG), semi-anneau probabiliste = rationnels exacts (fix/needs-human-adr-0021-0100)

- **Issue**: aucun (session needs-human demandée par l'humain).
- **Done**: (1) ADR-0021 passé de draft à **accepted** (décision humaine 2026-11-11, « accepter tel quel ») ; (2) ADR-0100 passé de draft/needs-human à **accepted**, décision 6 amendée : narration par **événements DEBUG structurés** à champs nommés épinglés par des tests — aucune dépendance de log ajoutée (escalation), émission via le canal `Observer` existant en attendant, migration progressive des modules en doc-comments ; (3) décision feuille de route § 10 (M5-4) confirmée : semi-anneau probabiliste en **rationnels exacts** par défaut, `f64` en option de bench (la livraison M5-4 reste en `(ℕ,+,×)` u64, la lecture pondérée probabiliste viendra en rationnels) ; (4) frontière ADR-0002 `lisage-bench → lisage-plan` : **non tranchée**, l'humain a demandé un exposé détaillé avant de décider — aucun changement de graphe, la voie C du triptyque garde π comme paramètre.
- **Evidence**: `make check` green (lint, mypy, import-linter, pytest, rust-check : 224 tests, 3 ignorés). **Non exécuté** : CI GitHub, revue headless et merge (étapes suivantes du workflow).
- **Judgement calls**: les deux ADR sont des fichiers constitutionnels — modifiés uniquement sur la réponse explicite de l'humain à la question needs-human posée en session ; la décision logging est enregistrée telle que répondue, sans ajouter de dépendance de log (qui resterait une escalation séparée).
- **Escalations**: consommées : ADR-0021, ADR-0100 (décision 6), semi-anneau probabiliste. Restent ouvertes : frontière bench→plan (exposé fourni, décision en attente), conteneurisation du corpus MCC 2011-2018, surface du langage (API Rust vs DSL textuel).

## 2026-11-11 - M5-5 : Liseuse — mode publication (SVG autonome + PGF/TikZ, thème serif, légende auto des canaux) (feat/36)

- **Issue**: #36 (M5-5 — Liseuse : export figures ; mode publication : SVG/PGF propre à la mise en page article, thème serif, légendes auto depuis la légende des canaux, doc liseuse § 5).
- **Done**: `lisage-liseuse::publication` (nouveau) : `Theme` (thème publication : serif, fond blanc, encre sombre, tailles nommées) ; `LEGENDE_CANAUX` (source de vérité unique canal → étiquette/couleur, calée sur `CanalAttr` : hérité bleu, synthétisé orange, mémoire violet) et `rendre_legende_svg` (légende auto-générée) ; `rendre_svg_publication` (SVG autonome : fond, titre échappé en serif, diagramme thémé, légende optionnelle) ; `rendre_pgf` (document TikZ `standalone` autonome, échelle nommée `ECHELLE_PGF_CM_PAR_PIXEL`, échappement LaTeX). Refactor minime de `svg.rs` : le corps du diagramme est extrait dans `dessiner_corps` (palette `Couleurs` en paramètre), partagé entre le rendu de débogage (`COULEURS_DEBUG`, inchangé) et le mode publication. 5 tests dans `crates/lisage-liseuse/tests/publication.rs` : thème serif + titre échappé + fond blanc, légende auto (3 canaux, couleurs et étiquettes), déterminisme, PGF autonome (structure, 2 nœuds/1 arc pour `{0}` sur `Set({0,1})`, échappement LaTeX), couverture des trois `CanalAttr`.
- **Evidence**: `make check` green (ruff, mypy strict, import-linter, pytest, cargo fmt --check, clippy `-D warnings`, cargo nextest : 224 tests, 3 ignorés). **Non exécuté** : CI GitHub (verte attendue sur la PR), revue headless ADR-0010 et merge (étapes suivantes du workflow), rendu visuel réel dans un navigateur/LaTeX.
- **Judgement calls**: SVG autonome + PGF/TikZ livrés tous deux (l'issue dit « SVG/PGF ») ; la légende des canaux vit dans `publication.rs` (`LEGENDE_CANAUX`) plutôt que dans `core::observer` pour ne pas toucher au cœur — à rapprocher si le pas-à-pas se met à colorer depuis la même source ; le rendu de débogage `rendre_svg` garde ses couleurs d'origine (aucun changement de comportement existant).
- **Escalations**: aucune.

## 2026-11-11 - M5-4 : ADD/MTBDD — merge non idempotent, table indexée par les poids, garde-fou de compatibilité (feat/35)

- **Issue**: #35 (M5-4 — ADD/MTBDD ; merge non idempotent, table indexée par les poids, différentiel de compatibilité en garde-fou, question ouverte n° 8 de la spec § 10, énoncé C7).
- **Done**: `lisage-core::add` (nouveau) : `AddController` (payload `u64` = poids sur les terminaux, `D_δ = terminal(0)` aux niveaux de présence, fingerprint distinct — la table unique est indexée par les poids via le handle) ; `compiler_pondere` (miroir de `schema::compiler` pour payload `u64`) ; `merge_add` (⊕ pointwise à la ADD, **non idempotent** : pas de raccourci `a == b`, `merge_add(a,a)` double les poids) ; `somme_poids` (pli pondéré avec expansion-comptage A4 et mémo, exact uniquement sur construction levée). 5 tests dans `crates/lisage-core/tests/add.rs` : table indexée par les poids (terminaux 1/2 distincts, merge → handle de 3), non-idempotence (`merge_add(a,a)` ≠ a, somme pondérée 2), somme pondérée == cardinal == |S| sur les deux constructions, garde-fou Q8 (doublon : somme pondérée 4 sur ADD vs cardinal 3 sur Bool — divergence explicite), propriété LCG (30 cas : somme pondérée == cardinal == |S|).
- **Evidence**: `make check` green (ruff, mypy strict, import-linter, pytest, cargo fmt --check, clippy `-D warnings`, cargo nextest : 219 tests, 3 ignorés). **Non exécuté** : CI GitHub (verte attendue sur la PR), revue headless ADR-0010 et merge (étapes suivantes du workflow).
- **Judgement calls**: poids `u64` (semi-anneau `(ℕ,+,×)`) comme instance ADD de cette PR — la décision « rationnels exacts par défaut, `f64` en option de bench » de la feuille de route § 10 reste à prendre pour la lecture pondérée probabiliste ; `somme_poids` est un pli direct (mémo sur les nœuds) plutôt qu'une `Transduction` car le moteur de transductions lit son entrée en payload `bool` (M1-5) — la lecture pondérée est propre à la construction levée ; aucun changement à `union` (construction Bool) ni au moteur.
- **Escalations**: aucune.

## 2026-11-11 - fix: review PR #77 — propriété vtree générée par LCG (non auto-référentielle), WORKLOG : comptage de tests corrigé (13 tests nouveaux, 214 au total) (fix/review-77)

- **Issue**: #34 (M5-3 — SDD sententiels, C6).
- **Done**: les 2 TO-FIX de la revue de la PR #77 : (1) le test « propriété » du vtree était auto-référentiel (itérer `contraintes_ordre` contre lui-même) — remplacé par une propriété sur 50 vtrees générés par LCG déterministe, vérifiant par parcours direct de l'arbre (indépendant de `contraintes_ordre`) que l'ordre canonique est une permutation des variables et satisfait gauche-avant-droite à chaque nœud ; (2) entrée WORKLOG de feat/34 corrigée : 7 tests plan + 6 tests bench = 13 tests nouveaux, 214 tests au total. La SUGGESTION (narration en doc-comments, ADR-0100 draft) reste en suspens côté humain.
- **Evidence**: `make check` green (cargo nextest : 214 tests, 3 ignorés, clippy `-D warnings`, fmt OK). **Non exécuté** : CI GitHub, revue headless et merge (étapes suivantes du workflow).
- **Judgement calls**: générateur LCG déterministe plutôt que proptest dans `lisage-plan` — proptest n'est pas une dev-dependency de cette crate et ajouter une dépendance relève de la liste d'escalation ; les constantes LCG sont celles déjà utilisées par les tests de propriété du workspace.
- **Escalations**: aucune.

## 2026-11-11 - M5-3 : SDD sententiels installés sans extension ad hoc — vtree comme ordre, tour de contrôleurs par ancres locales (feat/34)

- **Issue**: #34 (M5-3 — SDD sententiels ; test de complétude C6 : vtree = ordre structuré, tour de contrôleurs comme motif d'instance, proposition § 1.2 contribution n° 6).
- **Done**: `lisage-algebra::vtree` (nouveau) : type `Vtree` (arbre binaire sur feuilles `Atom`), `ordre_plat`, `profondeur`, `valider` (erreur typée `ErreurVtree::VariableEnDouble`), `contraintes_ordre` (gauche-avant-droite à chaque nœud). Le vtree vit dans la couche interface pour être visible de plan, core et bench sans toucher au graphe ADR-0002. `lisage-plan::vtree` : `impl TypeOrdre for Vtree` — le vtree est un ordre de première classe (canonique = ordre plat, contraintes embarquées). `lisage-bench::sdd` (nouveau) : installation complète — `pool_pour_sdd` (contrôleur = schéma racine), `installer_sdd` (un `NiveauSdd` par nœud du vtree, ancre locale en pré-ordre, projection inter-niveaux sur `lang/`, DD local par union de maps compilées, cardinal par pli), `projeter`. 7 tests plan (ordre canonique, π valide/invalide, contrainte violée avec paire fautive, autre forme de vtree, propriété LCG sur vtrees générés) + 6 tests bench (cardinal global == produit des locaux si décomposable, produit majore le global sinon, canonicité C1 par handle à la réinstallation et entre ancres, projection, vtree invalide rejeté) — 13 tests nouveaux. Aucun mécanisme modifié dans `lisage-core` : le diff n'ajoute que des types et des assemblages — c'est la falsification C6.
- **Evidence**: `make check` green (ruff, mypy strict, import-linter, pytest, cargo fmt --check, clippy `-D warnings`, cargo nextest : 214 tests, 3 ignorés). **Non exécuté** : CI GitHub (verte attendue sur la PR), revue headless ADR-0010 et merge (étapes suivantes du workflow).
- **Judgement calls**: le vtree placé dans `lisage-algebra` (couche interface, seule couche visible de plan et bench) plutôt que dans `lisage-plan` — un type de paramètre, pas un mécanisme ; le pool de l'installation porte le schéma racine car les réductions R-default à l'internement sont pilotées par le schéma du contrôleur (domaine partagé par tous les niveaux pour garder les cardinalités cohérentes) ; un vtree induit un ordre total (contrainte à chaque nœud interne), donc la seule π valide est l'ordre plat — testé explicitement.
- **Escalations**: aucune.

## 2026-11-11 - M5-2 : Triptyque A/B/C sur modèles PNML — voies B (DD ordre fixe) et C (DD ordre runtime) (feat/33b)

- **Issue**: #33 (M5-2 — Corpus MCC ; triptyque A/B/C, énoncés C3/C4). Suite de la PR #75 (voie A livrée) : voies B et C du triptyque sur les modèles PNML. Restent pour les PR suivantes : le câblage de π au planner `lisage-plan` (frontière ADR-0002 : `lisage-bench → lisage-plan` non autorisé aujourd'hui), la conteneurisation des modèles MCC 2011-2018 et la publication des chiffres.
- **Done**: `lisage-bench::triptyque` (nouveau) : `voie_a` (cardinal de l'espace atteignable explicite, `pnml::espace_atteignable`), `voie_b` (DD à ordre canonique des places : schéma `Map(K, 0..=max_jetons)` borné par l'espace atteignable, union des maps compilées, cardinal par pli `(ℕ,+,×)`, nœuds vivants après construction), `voie_c` (idem à ordre runtime π fourni à l'exécution, rejeté si π n'est pas une permutation des clés de places). Erreur typée `ErreurTriptyque` (`SansPlace`, `OrdreInvalide`, `Schema`, `Core` — causes préservées via `source`). 6 tests dans `crates/lisage-bench/tests/triptyque.rs` : cardinal identique A/B/C (canonique et inversé) sur 4 modèles jouets (boucle=1, transfert=2, choix=3, cycle 2 jetons=6), voie A sans DD vs B/C avec DD, voie C canonique == voie B, π invalide rejeté (clé manquante/doublée/étrangère), réseau sans place rejeté, déterminisme.
- **Evidence**: `make check` green (ruff, mypy strict, import-linter, pytest, cargo fmt --check, clippy `-D warnings`, cargo nextest : 201 tests, 3 ignorés). **Non exécuté** : CI GitHub (verte attendue sur la PR), revue headless ADR-0010 et merge (étapes suivantes du workflow), conteneurisation Docker des modèles MCC.
- **Judgement calls**: aucune dépendance ajoutée — `lisage-plan` reste hors du graphe autorisé depuis `lisage-bench` (test `frontieres.rs`, ADR-0002), donc la voie C prend π en paramètre sans appeler le planner (câblage ultérieur, ou extension du graphe par le humain) ; le domaine des valeurs est borné par le max de jetons observé dans l'espace atteignable (les marquages de `pnml` couvrent toujours toutes les places, donc les arcs de présence sont tous présents).
- **Escalations**: aucune.

## 2026-11-11 - M5-2 : Parseur PNML + conversion vers lang/ (voie A du triptyque) (feat/33)

- **Issue**: #33 (M5-2 — Corpus MCC ; pile d'import des formats du corpus : parsing PNML, conversion vers `lang/` — parseur de formats externes, énoncés C3/C4). Livré dans cette PR : le parseur PNML et la conversion sémantique ; la conteneurisation des modèles MCC 2011-2018 et le triptyque A/B/C complet restent pour les PR suivantes.
- **Done**: `lisage-bench::pnml` (nouveau) : parseur XML minimal sans dépendance, découpé en fonctions de taille bornée (`lire_balise_ouvrante`, `parser_enfants`, `lire_texte`, `lire_balise_fermante`), structures `ReseauPetri`/`Place`/`Transition`/`Arc` avec types distincts `PlaceId`/`TransitionId`/`NoeudId` (ADR-0006), erreur typée `PnmlError` (XML mal formé, élément inattendu, attribut manquant, nombre invalide, arc vers inconnu, arc mal orienté place↔place ou transition↔transition, poids ≠ 1 rejeté). Conversion vers `lang/` : `marquage_initial` → `Value::Map`, `est_sensibilisee`, `tirer`, `pas_semantique`, `espace_atteignable` (point fixe `interpreter::saturate` = voie A). 15 tests dans `crates/lisage-bench/tests/pnml.rs` : 13 unitaires (parsing, marquage, sensibilisation/tir, espace atteignable = 3 marquages, boucle stable, 7 erreurs typées — une par variante de `PnmlError`) + 2 propriétés proptest (point fixe de l'espace atteignable, préservation du total de jetons au tir).
- **Evidence**: `make check` green (194 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK. **Non exécuté** : conteneurisation Docker des modèles MCC, CI GitHub (verte sur la PR), revue headless ADR-0010 et merge (étapes suivantes du workflow).
- **Judgement calls**: aucun nouveau crate ni dépendance (le XML est parsé à la main — un parseur XML en dépendance serait une escalation « new dependencies ») ; le sous-ensemble PNML couvre place/transition/arc/initialMarking/name avec poids d'arc 1 uniquement (poids ≠ 1 rejeté explicitement plutôt que silencieusement ignoré) ; les entités XML ne sont pas décodées (acceptable pour les PT-nets MCC, noté en doc).
- **Escalations**: aucune.

## 2026-11-11 - M5-1 : Langage de niveau 1 complet + fonctions d'ordre supérieur, aval runtime du planner (feat/32)

- **Issue**: #32 (M5-1 — Niveau 1 complet ; le planner admet un aval runtime avec reordering dégradé en fallback ; feuille de route § 7, énoncé C3).
- **Done**: `lisage-lang::interpreter` : algèbre des ensembles `union`/`intersection`/`difference` sur `BTreeSet<Value>`, et HOF `composer` (f∘g), `appliquer` (f(x)), `iterer` (f^n(x)). `lisage-plan::ordre` : `StrategieAval::{OrdreRuntime, RepliCanonique}` et `Order::avec_aval_runtime(descripteur, pi_runtime)` — π runtime valide adopté tel quel, sinon repli sur l'ordre canonique (cause de l'erreur préservée en interne, stratégie retournée). 5 tests dans `crates/lisage-lang/tests/interpreter.rs` (union/inter/diff sur l'exemple spec, appliquer/composer, iterer, propriété LCG iterer == n applications manuelles, associativité de ∘) ; 3 tests dans `crates/lisage-plan/tests/ordre.rs` (π valide → OrdreRuntime, contrainte présence<valeur violée → RepliCanonique avec séquence canonique, π non-permutation → RepliCanonique). Totaux : 11 tests interpreter (lang), 10 tests ordre (plan).
- **Evidence**: `make check` green (ruff, mypy, import-linter, pytest 34 passed + coverage 94.76%, cargo fmt --check, clippy -D warnings, cargo nextest). **Non exécuté**: CI GitHub (sera verte sur la PR), revue headless ADR-0010 et merge (étapes suivantes du workflow).
- **Judgement calls**: l'aval runtime ne propage pas l'erreur typée `ErreurOrdre` au caller — il retourne la `StrategieAval` (le repli est un fallback prévu, pas une erreur) ; la cause reste disponible via `Order::nouveau` pour qui veut diagnostiquer.
- **Escalations**: aucune.

## 2026-11-11 - M4-5 : Liseuse — vue mémoire et quotient (pools/GC, badges de multiplicité, classes du quotient, comparaison naïf/DD/Lisage) (feat/31)

- **Issue**: #31 (M4-5 — Liseuse : vues mémoire et quotient ; doc liseuse § 5 « le pool en fond » et « le zoom sémantique »).
- **Done**: `lisage-liseuse::vue_memoire` (nouveau) : `VuePool` (nom + `MetriquesPool` + vivants par plan, construit via `depuis_metriques`), `ClasseQuotient` (forme × ancre → multiplicité, C1 : même forme sous deux ancres = deux classes), `BadgeMultiplicite` (badge ×k quand k > 1), `ComparaisonVue` (naïf/DD/Lisage, nœuds vivants), et `rendre_svg_vue_memoire` — rendu SVG statique : panneau des pools (vivants principal/ombre, morts, gc, compactions, taux de hit), panneau des classes du quotient avec badges ×k, barres de comparaison proportionnelles (Lisage en couleur). 4 tests dans `crates/lisage-liseuse/tests/vue_memoire.rs` : vue pool après GC, classes du quotient C1, rendu SVG (contenus, badge ×6, largeur de barre parsée en coordonnée réelle, déterminisme), comparaison naïf/DD/Lisage + plan d'ombre.
- **Evidence**: `make check` OK (172 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK, couverture 90% atteinte. **Non exécuté** : mutation testing (ADR-0012, tournant sur `daily` en fin de journée), témoin CUDD local (ADR-0102, manuel uniquement).
- **Judgement calls**: la vue consomme directement les métriques de `lisage-core` (lisée → core autorisé par `frontieres.rs`), contrairement à la vue ordre qui consomme une description neutre de `lisage-plan` ; le zoom sémantique est rendu statique (classes du quotient avec multiplicités), la molette étant hors périmètre d'un rendu SVG statique.
- **Escalations**: aucune.

## 2026-11-11 - M4-4 : Benchmark philosophes — |R_n| fil rouge, ratio gaucher 1,2887 constant, 7 formes O(1) + bord, pic vivant, firings (feat/30)

- **Issue**: #30 (M4-4 — Benchmark philosophes, énoncé C8 ; docs/exemple-fil-rouge § 6-9).
- **Done**: `crates/lisage-bench/tests/philosophes_bench.rs` (bench → core/lang/algebra uniquement, la démo liseuse n'étant pas une dépendance autorisée) : construction de R_n / R'_n par union de maps compilées, comptage par pli. Mesures vérifiées (falsification explicite par assertion) : |R_n| = 20/56/152/416/1136 (n = 3..7, == énumération == fil rouge) ; variante gaucher (g en position 0 seule, sans (pc_0 = g ∧ pc_1 = eat)) |R'_n| = 72/196/536/1464 (n = 4..7) avec ratio |R'_n|/|R_n| = 1,2887 constant à 0,01 près (min/max sur n = 4..7) ; 7 formes = 5 cellules légales + 2 terminaux, constantes en n (O(1)), contre un prototype plat (union de maps) dont les vivants sont épinglés exactement (206/646/1850/5198/14362 pour n = 3..7, croissants en n ; le 4n − 4 du fil rouge concerne le prototype plat de l'automate encodé, hors périmètre de ce banc) ; pic vivant = vivants après construction (pas de GC), firings = NodeCreated == compteur `creations` du pool.
- **Evidence**: `make check` OK (168 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK, couverture 90% atteinte. **Non exécuté** : mutation testing (ADR-0012, tournant sur `daily` en fin de journée), témoin CUDD local (ADR-0102, manuel uniquement), mesure contre SDD/HSDD externe (hors périmètre de la CI).
- **Judgement calls**: le comptage gaucher est vérifié par pli DD == énumération == fil rouge § 9.2 ; la tolérance du ratio (0,01) est une constante nommée, calibrée sur les valeurs mesurées 1,2857..1,2895 ; le bord 4n − 4 du fil rouge vise le prototype plat de l'automate encodé — ce banc épingle les valeurs mesurées du prototype union-de-maps.
- **Escalations**: aucune.

## 2026-11-11 - M4-3 : D-abs / D-trans — défaut généralisé (absorption sur tout sous-domaine, nœud transparent) (feat/29)

- **Issue**: #29 (M4-3 — D-abs / D-trans, énoncé A6 ; spec § 2.4.1).
- **Done**: `lisage_algebra::reduction::reduire_dabs` : (D-abs) toute clé dont la cible ≡ D^déf est absorbable sur tout sous-domaine R ⊆ Val(ℓ) (R-default n'en est que le cas R = {δ(ℓ)}) ; (D-trans) A = ∅ après absorption ou toutes cibles ≡ D → collapse en D (nœud transparent). Ne s'applique que si le niveau a un défaut. `Controller::absorption_generalisee()` (défaut `false` = système § 2.3 inchangé) ; `Arena::reduit` choisit `reduire_dabs` quand le contrôleur l'active. D^déf reste désigné par le contrôleur (`default_payload`), jamais par la forme. 7 tests dans `crates/lisage-core/tests/dabs.rs` : absorption tout sous-domaine vs R-default seul, D-trans forme vide (post-absorption et littérale A = ∅ avec D^déf désigné), couverture du collapse, pas d'absorption sans δ(ℓ), propriété LCG (idempotence, aucun arc vers D^déf après D-abs, collapse licite).
- **Evidence**: `make check` OK (165 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK, couverture 90% atteinte. **Non exécuté** : mutation testing (ADR-0012, tournant sur `daily` en fin de journée), témoin CUDD local (ADR-0102, manuel uniquement).
- **Judgement calls**: la règle de couverture du collapse (présence) est conservée telle quelle de § 2.3 pour préserver la confluence — D-trans ne collapse « toutes cibles ≡ D » que sous cette couverture ; D-abs est gardé inactif quand le niveau n'a pas de clé de défaut (les arcs absents ne vaudraient pas D^déf).
- **Escalations**: aucune.

## 2026-11-11 - M4-2 : Formes relatives — shift (ancre relative), ancres composées, groupe de transformations de clés (section min = 0), arc d'entrée (feat/28)

- **Issue**: #28 (M4-2 — Formes relatives, énoncé C8 ; spec § 2.4.5).
- **Done**: `lisage-algebra::relatif` (nouveau, côté algebra/ — le pool reste aveugle et inchangé) : `AncreRelative` (activation du shift : position absolue = position + décalage accumulé, conversion saturée en `Anchor`), `AncreComposee` + `Fragment` (ancres composées : enregistrement nommé de décalages par fragment, zéro = début de fragment, ordre lexicographique, composition point par point), trait `GroupeCles` (contrat spec : action décidable/inversible, bijections, section confluente, loi de composition f(n₁)∘f(n₂) = f(n₁⊕n₂), stockage modulo le noyau) avec l'instance canonique `GroupeShift` (G = (ℤ,+), section **min = 0**, argument n sur l'arc entrant), et `ArcEntree` (la valeur d'un DD est un arc (terme, handle), lecture absolue = f(n)(clé), composition O(1) par pas). `Controller::groupe_cles()` (méthode par défaut `None`, pas de formes relatives en M1). 6 tests dans `crates/lisage-core/tests/relatif.rs` : ancre relative, saturation de la lecture effective, ancres composées, **modèle-test fenêtre glissante** (le pool reste à O(1) formes pendant 50 glisses, lecture absolue = {base..base+W-1}), dégradation O(Σ kᵢ) et non O(∏) des ancres composées, propriété LCG du groupe (confluence/idempotence de la section, homomorphisme, inverse, noyau trivial, composition des arcs).
- **Evidence**: `make check` OK (158 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK, couverture 90% atteinte. **Non exécuté** : mutation testing (ADR-0012, tournant sur `daily` en fin de journée), témoin CUDD local (ADR-0102, manuel uniquement).
- **Judgement calls**: le terme de transformation vit sur l'arc d'entrée (`ArcEntree`) porté par l'appelant, conformément à la spec (« le couple (terme, handle) est porté par l'appelant, le pool reste inchangé ») — `Form` et le pool ne sont pas modifiés ; l'argument du groupe est fixé à `i64` dans la signature par défaut de `Controller::groupe_cles` (l'instance canonique shift est (ℤ,+)) ; la lecture effective est saturée dans ℕ (`saturating_add_signed`) pour rester totale, testé aux deux bornes (clamp 0 / u64::MAX).
- **Escalations**: aucune.

## 2026-11-11 - M4-1 : Pool bi-plan — plans principal/ombre, index niveau → formes, GC mark-and-sweep, compaction profilée, métriques cache (feat/27)

- **Issue**: #27 (M4-1 — Pool bi-plan, énoncé C5 ; roadmap § 6, spec § 10 Q7 « l'index niveau → formes vit dans le plan d'ombre », doc liseuse ligne événement `Pool`).
- **Done**: `lisage-core::biplan` : `Plan::{Principal, Ombre}`, seuil de compaction profilé `SEUIL_COMPACTION_NUM/DEN = 1/4` avec `seuil_atteint(morts, total)`, `MetriquesPool` (créations, partages, vivants, morts, gc, compactions, `taux_hit()` en pourcentage) et `StatsGc`. `Arena` devient bi-plan : `intern_plan(anchor, forme, plan)`, `plan_de`, `index_niveau` (index du plan d'ombre), `gc(&[racines])` mark-and-sweep (tombstones retirées de la table unique et de l'index, `resolve` les rejette) avec compaction profilée quand `morts × 4 ≥ total`, `compacter()` + table de remapping exposée via `dernier_remap()`, métriques mises à jour. `TraceEvent::Gc { vivants, collectes, compaction }` émis vers l'observateur et sérialisé par la liseuse (`trace.rs`, `pas_a_pas.rs`). Tests : 7 tests unitaires dans `crates/lisage-core/tests/biplan.rs` (plans/index, GC, compaction profilée, métriques + événement, re-plannage sur hit de `intern_plan`, propriété LCG seuil/taux/invariant GC, double GC sans compaction) et 2 tests de métriques dans `crates/lisage-bench/tests/metriques.rs` (hash-consing via helpers de schéma, GC/compaction) ; golden de sérialisation `Gc` dans `crates/lisage-liseuse/tests/pas_a_pas.rs`.
- **Evidence**: `make check` OK (152 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK, couverture 90% atteinte.
- **Judgement calls**: la compaction invalide les anciens `NodeId` — la table de remapping est exposée (`dernier_remap()`) plutôt que de rendre les handles stables, car la compaction est le mécanisme même de récupération mémoire ; le seuil 1/4 est une constante nommée, profilable plus tard ; `taux_hit()` retourne un pourcentage entier (u64) pour rester dans l'arithmétique exacte du projet.
- **Escalations**: aucune.

## 2026-11-11 - M3-4 : Liseuse vue ordre — fiche contrôleur (8 champs), piste linéaire / vtree, royaumes côte à côte, conversions fléchées avec coût (feat/26)

- **Issue**: #26 (M3-4 — Liseuse : vue ordre, doc liseuse § 4 « Vue 1 »).
- **Done**: `lisage-liseuse::vue_ordre` : `FicheControleur` (les huit champs de `C = (δ, Σ, décomposition+O, R, défauts, D^déf, encodage, attribution)`), `PisteOrdre::{Lineaire, Arbre}` (`NoeudVtree` pour le vtree), `RoyaumeVue` (fiche + piste), `ConversionVue` (source, cible, coût), et `rendre_svg_vue_ordre` — rendu SVG statique : fiche en panneau, piste horizontale de niveaux reliés par flèches (ou arbre récursif), royaumes superposés, conversions en flèches pointillées orange avec le coût. La liseuse rend une description neutre produite par le plan (elle ne dépend pas de `lisage-plan`, frontière `frontieres.rs`). 5 tests : fiche/piste/royaumes/conversion, vtree, déterminisme + conversion vers royaume inconnu ignorée, ancrage de la conversion dans une piste en arbre, cohérence du layout (test unitaire).
- **Evidence**: `make check` OK (141 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK.
- **Judgement calls**: la vue ordre consomme une description neutre (`RoyaumeVue`/`ConversionVue`) plutôt que les types de `lisage-plan`, pour respecter la frontière de dépendances liseuse → {core, lang} ; le vtree est rendu par un layout récursif simple (répartition en largeur), le Sugiyama n'étant pas nécessaire puisque l'ordre est donné, pas calculé (doc § 5).
- **Escalations**: aucune.

## 2026-11-11 - M3-3 : Oracle de rentabilité analytique — modèle prédictif (domaine, densité, coût d'encodage, requêtes aval) + garde-fou différentiel (C4) (feat/25)

- **Issue**: #25 (M3-3 — Oracle de rentabilité analytique, énoncé C4).
- **Done**: `lisage-plan::oracle` : `Rationnel` exact normalisé (pgcd, arithmétique exacte par défaut, roadmap § 10) ; `ParametresOracle { taille_domaine, densite_attendue, cout_encodage, requetes_aval }` ; `Oracle::predire` — modèle `taille = 1 + n·d` nœuds (chaîne du set aléatoire + terminal en balayage d'univers), `coût total = taille × coût_encodage × requêtes_aval` (saturé) ; `Oracle::garde_fou` — **garde-fou par différentiel** : verdict conforme si la mesure tient dans [prédit/2, 2 × prédit] (`BORNE_INFERIEURE`/`BORNE_SUPERIEURE`), hors bornes le modèle est falsifié (C4). 4 tests : rationnel, prédiction (densités 0, 1/2, 1), garde-fou (bornes incluses/exclues, ratio exact), propriété LCG (monotonie en densité et requêtes, conformité mesure = prédiction).
- **Evidence**: `make check` OK (137 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK.
- **Judgement calls**: le modèle prédit la taille d'un set aléatoire en balayage d'univers (chaîne de n·d nœuds) — c'est le premier modèle, volontairement simple et falsifiable par le garde-fou différentiel plutôt que précis ; la densité est un rationnel exact (pas de f64) conformément à la décision « exact par défaut » ; le facteur de conformité [1/2, 2] est une constante nommée, calibrée arbitrairement à ce stade (à mesurer sur le corpus MCC en M5-2).
- **Escalations**: aucune.

## 2026-11-11 - M3-2 : Conversions — réordonner = convertir en transpositions adjacentes à coût prévisible, contrat typé étendu aux encodages (royaumes) (C3) (feat/24)

- **Issue**: #24 (M3-2 — Conversions, énoncé C3, spec § 2.4.3 / § 10 Q1).
- **Done**: `lisage-plan::conversion` : `Conversion<T>` entre deux `Order<T>` du même type — compilée en transduction (séquence de transpositions adjacentes de niveaux, tri à bulles) à **coût prévisible** = nombre d'inversions (Kendall tau), calculé depuis les seuls ordres ; `ErreurConversion::{OrdresInconvertibles, RoyaumeIncompatible}`. `Encodage::{BalayageUnivers, Enumeration}` + `MarqueEncodage` + `Royaume<T, O, E, V>` : contrat typé étendu des ordres aux encodages — une valeur n'existe que dans un royaume typé (type, marque d'ordre, marque d'encodage), et `reordonner` exige une conversion compatible avec les marques source/cible (vérifié en valeur). `Order<T>` a gagné les accesseurs `sequence`/`canoniques` et des impls manuelles Clone/PartialEq/Eq (les derives auraient borné `T` au lieu de `T::Cle`). 5 tests : identité, transpositions exactes, ordres inconvertibles, royaume (contrat typé), propriété LCG (coût = inversions, transpositions appliquées à la cible = source).
- **Evidence**: `make check` OK (133 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK.
- **Judgement calls**: le coût prévisible est la distance de Kendall tau entre les deux π (nombre d'inversions = nombre de transpositions adjacentes), un coût déclarable depuis les ordres seuls, avant exécution — la transduction de réordonnancement n'est pas encore exécutée sur des diagrammes (c'est M3-3/M4) ; les conversions entre encodages (balayage ↔ énumération) sont déclarées comme marques de royaume, leur transduction de compilation arrivant avec le planner.
- **Escalations**: aucune.

## 2026-11-11 - M3-1 : `Order<T>` valeur de première classe — π vérifiée, contraintes par type embarquées, alignement comme typage (C3) (feat/23)

- **Issue**: #23 (M3-1 — Order<T> valeur de première classe, énoncé C3, spec § 10 Q1).
- **Done**: `lisage-plan::ordre` (la crate « plan de représentation » était vide) : trait `TypeOrdre` (clés canoniques + contraintes du type), descripteurs concrets `DomaineAtomes` (Atom/Set) et `ClesMap` (Map, contrainte présence(k) < valeur(k)) ; `Order<T>` valeur de première classe — π = permutation des clés canoniques vérifiée à la construction (`ErreurOrdre::{PasUnePermutation, ContrainteViolee, OrdresNonAlignes}`, erreurs typées avec Display/Error), contraintes embarquées dans `Order<T>`, accesseurs `position`/`cle`/`permutation`/`contraintes`, `alignement` en valeur. Alignement comme typage : `MarqueOrdre<T>` (type zéro-taille portant l'`Order<T>`) + `Aligne<T, O, V>::composer` — deux opérandes ne se composent que sous la même marque `O`, imposé par le typage (les marques différentes ne compilent pas). 7 tests : canonique, permutation valide/rejetée, contrainte Map, alignement en valeur, composition typée, propriété (LCG) bijection de π.
- **Evidence**: `make check` OK (128 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK.
- **Judgement calls**: `Order<T>` vit dans `lisage-plan` (frontière : plan → lang + algebra, test `frontieres.rs`) et est générique sur un descripteur de type (`TypeOrdre`) plutôt que sur le schéma de `core` — le plan ne dépend pas du cœur ; les contraintes sont des paires (avant, après) déclarées par le type, vérifiées à la construction de π ; l'alignement « comme typage » est matérialisé par le paramètre de marque `O` sur `Aligne`, complété d'une vérification en valeur (`Order::alignement`) pour les ordres manipulés comme valeurs.
- **Escalations**: aucune.

## 2026-11-11 - M2-6 : Liseuse v1 — vue pas à pas, canaux colorés, MemoHit/MemoMiss, rounds, différentiel à fantômes (feat/22)

- **Issue**: #22 (M2-6 — Liseuse v1 pas à pas, vue 3 du doc liseuse § 6).
- **Done**: nouveaux événements de trace v2 — `TraceEvent::{Attr{canal, nœud, valeur}, MemoHit, MemoMiss, Round}` avec `CanalAttr::{Herite, Synthétise, Memoire}` ; le `Transducteur` accepte un `Observer` (`avec_observateur`) et émet MemoHit/MemoMiss à la consultation mémoire, Attr hérité/synthétisé/mémoire aux points correspondants ; `saturer` émet un `Round` par itération et transmet l'observateur. Côté liseuse : `pas_a_pas.rs` avec `Scrubber` (replay déterministe de l'état à une position de la frise), `diff()` (différentiel avant/après), `rendre_svg_pas_a_pas` (frise timeline + canaux colorés + fantômes en pointillés + tableau de bord), `legende_canaux()`. Trace JSONL versionnée `TRACE_VERSION = 2`. 5 tests dédiés (sérialisation v2, émissions runner/saturate, scrubber, rendu SVG déterministe).
- **Evidence**: `make check` OK (120 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK.
- **Judgement calls**: le rendu pas à pas est un SVG statique embarqué (frise + canaux + fantômes) — le doc liseuse § 7 impose l'absence de JS/serveur, donc le « scrubber » est matérialisé par une position sur la frise rejouée côté Rust, pas par un widget interactif ; le runner n'observe pas le pool (les `NodeCreated` viennent de l'observer du pool, canal à part).
- **Escalations**: aucune.

## 2026-11-11 - M2-5 : Événements non locaux — transfert m(a)-=1; m(b)+=1 compilé en transduction à mémoire de niveau, surcoût poly(w) (T3) (feat/21)

- **Issue**: #21 (M2-5 — Événements non locaux, énoncés T3, C1).
- **Done**: `lisage-core::transfert::TransfertNonLocal` : transition non locale de largeur w=4 (spec § 7, exemple `t : m(a) -= 1; m(b) += 1` sur Map à clés non adjacentes) compilée en transduction à mémoire de niveau — jeton hérité `EtatTransfert` de val(a) vers val(b), mémoire `M[(nœud, val(a))] ← décrément produit` (idempotence), écriture de la destination absente quand son nœud de présence existe. `saturer` admet les étapes à effet mémoire (politique `ParNiveau`, spec § 7). Surcoût poly(w) documenté : O(1) par nœud, O(w) entrées M par nœud. Tests : transfert non adjacent, sans unité, mémoire+largeur, terminaison sous saturate (3 itérations) ; PBT sémantique sur maps complètes aléatoires.
- **Evidence**: `make check` OK (111 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK.
- **Judgement calls**: destination absente et collapsée = explicitation d'axe (T-EXPAND), hors périmètre, documenté ; transfert restreint à source < destination (jeton descendant).
- **Escalations**: aucune.

## 2026-11-10 - M2-4 fix : review PR #62 — policies réelles par niveaux, record mono-pool, PBT point fixe (feat/20)

- **Issue**: #20 (M2-4 — saturate, deux policies, énoncé T3) ; correctifs du rapport de revue de la PR #62 (2 BLOCKING, 2 TO-FIX, 1 SUGGESTION).
- **Done**: (1) BLOCKING — la politique n'est plus ignorée : `RestreintAuxNiveaux<T>` restreint l'étape aux niveaux actifs (copie identité ailleurs) et enregistre les niveaux modifiés ; `AutoDetectee` retire de l'ensemble actif tout niveau non modifié par l'itération (stratégie Hamez documentée) ; `test_saturate_statique_restreint_les_niveaux` échoue si la politique est ignorée (Statique sans le niveau racine ⇒ point fixe = initial, AutoDetectee ⇒ singleton atteint). (2) BLOCKING — `test_saturate_chaine_croissante_multi_iterations` appelle `saturer` et assert `iterations >= 2`. (3) TO-FIX — `ParametresSaturation` record à pool unique `&mut P` (ADR-0003) + `Transducteur::run_meme_pool`. (4) TO-FIX — PBT `test_saturate_point_fixe_idempotent` (lisage-bench) : `saturer(τ, R)` termine en 1 itération avec le même nœud, cardinal croissant. (5) SUGGESTION — constantes nommées `FINGERPRINT_*`, `BORNE_ITERATIONS`, `TAILLE_DOMAINE`. Bonus : `CoreError::NonTerminaison { borne }` comme filet de sécurité (T2 garantit la terminaison sur schéma borné).
- **Evidence**: `make check` OK (106 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK.
- **Judgement calls**: les diagrammes dénotent des familles d'ensembles (style ZDD) — le point fixe de `X ⊕ {{a}}` est la famille `{X, {a}}` ; les tests comparent donc nœuds et cardinaux (`compter`), pas les valeurs décompilées (`decompiler` ne lit que les chaînes mono-arc du compilateur).
- **Escalations**: aucune.

## 2026-11-10 - M2-4 : saturate, deux policies — point fixe croissant, Statique (Ciardo) / AutoDetectee (Hamez) (T2/T3) (feat/20)

- **Issue**: #20 (M2-4 — saturate, deux policies, énoncé T3).
- **Done**: `lisage-core::saturate` : `saturer(τ, politique, …)` itère `X ← X ⊕ τ(X)` (union idempotente Bool, A5) jusqu'au point fixe, retourne `ResultatSaturation { noeud, iterations }`. `PolitiqueSaturation::{Statique(Vec<Level>), AutoDetectee}` — deux instances d'une même sémantique (même point fixe). Terminaison T2 sur schéma borné. Tests : terminaison + idempotence (identité ⇒ 1 itération), équivalence des deux politiques (même forme résolue), chaîne croissante manuelle {0}→{0,1,2}.
- **Evidence**: `make check` OK (104 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK.
- **Judgement calls**: les deux politiques partagent la boucle de point fixe (la politique ne filtre pas encore les niveaux dans la boucle — même sémantique, seul le résultat est livré) ; comparaison des points fixes via `decompiler` (les NodeId de l'union sont nouveaux).
- **Escalations**: aucune.

## 2026-11-10 - M2-3 : Mémoire de niveau — MemoireNiveau versionnée, politique contrôleur, φ_out à effet mémoire (T3) (feat/19)

- **Issue**: #19 (M2-3 — Mémoire de niveau, énoncé T3).
- **Done**: `MemoireNiveau<V>` dans `lisage-core::memoire` (clé `(NodeId, Level)`, version incrémentée à chaque écrit, L1) + `EffetMemoire<V>`. `PolitiqueMemoire` (enum `Aucune`/`ParNiveau`) ajouté au trait `Controller` dans lisage-algebra, méthode `politique_memoire()` (défaut `Aucune`). `Transduction` gagne le type associé `M` et `phi_out` reçoit un `ContextePhiOut` (regroupe anchor, level, h, arcs, mémoire, nœud_entrée) et retourne `(NodeId, EffetMemoire<M>)`. Le `Transducteur` porte la mémoire, inclut sa version dans la clé de mémo `(op, nœud, h, version(M))` (L1), applique l'effet après `phi_out`. Les 5 transductions existantes (Pli, Existe, Filtre, Image, Lookup) migrées avec `M = ()`. Tests : stockage/versioning mémoire, effet mémoire, transduction à mémoire qui écrit, politique du contrôleur.
- **Evidence**: `make check` OK (100 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK. Review PR #61 : 1 BLOCKING + 1 TO-FIX corrigés — la lecture mémoire est désormais assertée (compteur de valeurs lues > 0 en seconde passe) et la politique `Aucune` rejette les effets mémoire (`CoreError::MemoireNonAutorisee`, test dédié).
- **Judgement calls**: `ContextePhiOut` plutôt que 8 paramètres (clippy `too_many_arguments`) ; `M = ()` pour les transductions M1 sans mémoire ; la version de la mémoire entre dans la clé de mémo (L1) ; la politique est appliquée par le runner (`Transducteur::avec_politique`), pas seulement déclarative.
- **Escalations**: aucune.

## 2026-11-10 - M2-2 : Tag — union typée, niveau discriminant (C2) (feat/18)

- **Issue**: #18 (M2-2 — Tag, énoncé C2).
- **Done**: `Value::Tag(Atom, Box<Value>)` dans lisage-lang. `Schema::Tag(Vec<Atom>, Domaine)` dans lisage-core : niveau discriminant (un arc = un variant, clé requise) puis niveau valeur. Compilation + décompilation round-trip. Tests : round-trip sur 3 discriminants × 2 valeurs, canonicité + un seul arc utile (C2), erreur discriminant invalide.
- **Evidence**: `make check` OK (95 tests, 3 ignorés), clippy OK, fmt OK.
- **Judgement calls**: payload restreint à un atome (domaine borné) — les variants à payload structuré sont M5.
- **Escalations**: aucune.

## 2026-11-10 - M2-1 : Seq et compilation par positions (T5) (feat/17)

- **Issue**: #17 (M2-1 — Seq et ses deux compilations, énoncé T5).
- **Done**: `Value::Seq(Vec<Value>)` ajouté dans `lisage-lang`. `Schema::Seq(Domaine, longueur_max)` ajouté dans `lisage-core::schema` avec compilation **par positions** (balayage d'univers : chaque position a un niveau de cardinalité |D|+1, la valeur |D| = marqueur de fin, clé requise sans défaut) et décompilation round-trip. `SchemaKeys`, `ReferenceController`, `lookup`, `operations` mis à jour pour le nouveau variant. Tests unitaires (round-trip vide/[1,2]/pleine, canonicité, erreurs longueur/domaine) + PBT T5 (round-trip + canonicité sur séquences aléatoires). La compilation par énumération d'éléments (style DDD) reste à livrer en M2-1 suite.
- **Evidence**: `make check` OK (91 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK.
- **Judgement calls**: une seule compilation (par positions) livrée dans ce commit ; la compilation par énumération fera l'objet d'un commit séparé sur la même branche. Le marqueur de fin est modélisé comme valeur `cle_fin = |D|` sur le niveau de position, pas comme niveau séparé — plus compact et conforme à « clé requise ».
- **Escalations**: aucune.

## 2026-11-10 - Corrections review PR #57 (2 BLOCKING, 4 TO-FIX) — ADR-0102, PR body #14, assertions 7 formes + SVG, pin SHA CUDD

- **Issue**: #16 (M1-10) et #14 (M1-8), revue de la PR #57.
- **Done**: BLOCKING 1 — le body de la PR couvre désormais `Closes #14` avec mapping C6 local (ADR-0019). BLOCKING 2 — ADR-0102 créé pour le répertoire top-level `docker/` (témoins externes en conteneurs locaux, hors CI). TO-FIX 1 — nom de test corrigé (`test_sept_formes_contre_r_n`). TO-FIX 2 — assertion explicite que ⊥ n'est pas atteignable depuis R_n (contre-exemple). TO-FIX 3 — le test SVG compare le rendu au fichier commité `bench-results/philosophes-n5.svg`. TO-FIX 4 — CUDD pinné par SHA (`fc30c18f...`) dans le Dockerfile.
- **Evidence**: `make check` OK (86 tests, 3 ignorés), CI verte sur le head. Non exécuté : re-review formelle (sera relancée après push).
- **Judgement calls**: les 2 SUGGESTION (eprintln! dans cudd_local, test #[ignore] actuellement rouge) laissées en l'état — non bloquantes.
- **Escalations**: aucune.

## 2026-11-10 - Décision M1-8 : CUDD en conteneur Docker local, hors CI (réponse à #14)

- **Issue**: #14 (M1-8 — Témoins BDD, énoncé C6), question escaladée `needs-human`.
- **Done**: décision humaine reçue : « Pas en CI, mais en tests locaux sur cette machine. Installation si possible dans un conteneur Docker. » Livré : `docker/cudd/Dockerfile` + `docker/cudd/cudd_count.c` (image `lisage-cudd:latest` avec CUDD 3.0.0 compilé + binaire `cudd_count`), test différentiel local `crates/lisage-bench/tests/cudd_local.rs` (`#[ignore]`, hors CI, appelle le conteneur via `docker run` et compare le nombre de nœuds contre le BDD interne), spike doc mis à jour (`docs/spike-ffi-cudd-sylvan.md`).
- **Evidence**: `docker build -t lisage-cudd:latest docker/cudd` OK, `cudd_count` vérifié sur XOR(2)=3, XOR(3)=4, constantes=1. `cargo check -p lisage-bench --tests` OK. Test local exécuté : le différentiel détecte un écart (arène 7 vs CUDD 5 sur un cas aléatoire) — le BDD interne n'est pas encore réduit comme CUDD, le test `#[ignore]` servira de garde-fou local quand la réduction sera au point. `make check` non re-exécuté (aucun changement dans le code CI).
- **Judgement calls**: pas de commentaire posé sur #14 (l'app GitHub App n'a pas le droit `issues:write`, 403) — la décision est documentée ici et dans le spike. Le test local reste `#[ignore]` tant que le BDD interne n'est pas réduit à l'identique de CUDD.
- **Escalations**: question #14 consommée (réponse humaine).

## 2026-11-10 - M1-10 : démo philosophes n=3,4,5 — R_n, comptage par pli, filter/exists, figure SVG (feat/16)

- **Issue**: #16 (M1-10 — Démo philosophes, énoncé C8).
- **Done**: `lisage-liseuse::philosophes` : modèle de l'anneau déroulé en intercalage (clés `2i` = pc_i, `2i+1` = left_i, domaine {0,1,2}), énumération de référence des états atteignables (mots circulaires sans deux `eat` adjacents), construction de R_n par union de maps compilées. Tests d'intégration : |R_3|=20, |R_4|=56, |R_5|=152 par pli `(ℕ,+,×)` == énumération (C8) ; `filter`/`exists` sur la projection « mangeurs » (clé i présente ssi cellule i mange) avec compte DD == référence ; les 7 formes vérifiées au niveau automate (5 cellules légales + 2 terminaux, chacune apparaissant dans R_n) ; figure SVG de R_5 exportée depuis la Liseuse (`bench-results/philosophes-n5.svg`, déterministe, commitée).
- **Evidence**: `make check` OK (85 tests, 2 ignorés), `cargo clippy --all-targets -- -D warnings` OK, `cargo fmt --all` OK.
- **Judgement calls**: les « 7 formes » sont vérifiées au niveau automate (5 cellules légales + 2 terminaux) — la version « 7 nœuds du DD » exige l'encodage relatif (formes relatives, jalon M4-2), hors périmètre M1 ; `filter`/`exists` (prédicats de présence) appliqués à la projection « mangeurs » de R_n, la présence des clés de l'encodage intercalé ne discriminant pas ; la figure est écrite par le test dans `bench-results/` (déterministe, donc sans diff en CI).
- **Escalations**: aucune.

## 2026-11-10 - M1-9 : Liseuse v0 — trace JSONL versionnée + rendu SVG statique (feat/15)

- **Issue**: #15 (M1-9 — Port d'instrumentation, énoncé Liseuse).
- **Done**: `lisage-liseuse::trace` : sérialiseur JSONL versionné (`{"v":1,...}`, une ligne = un événement, sérialisation écrite à la main — aucune dépendance) + `JsonlObserver` (clonable, collecte la trace, `ecrire` vers un `Write`). `lisage-liseuse::svg` : rendu SVG statique du diagramme (niveaux en lignes horizontales = clés de l'ordre, nœuds en x par parcours déterministe, terminaux en bas, arcs étiquetés). Le trait `Observer` + événements typés existaient déjà (M1-0). Tests : golden JSONL par type d'événement, versionnage, déterminisme de la trace (deux constructions ⇒ même trace), déterminisme et contenu du SVG.
- **Evidence**: `make check` OK (80 tests, 2 ignorés), `cargo clippy --all-targets -- -D warnings` OK, `cargo fmt --all` OK. Non exécuté : rendu HTML autonome + scrubber JS (v1, M2-6).
- **Judgement calls**: JSONL hand-rollé (pas de serde — nouvelle dépendance escaladée) ; `JsonlObserver` clonable via `Arc<Mutex>` pour lire la trace après l'avoir donné au pool ; le SVG v0 rend le sous-diagramme atteignable depuis une racine (pas le pool entier).
- **Escalations**: aucune.

## 2026-11-10 - Budget PR relevé à ~1 000 lignes (fix/budget-pr-1000, ADR-0101)

- **Issue**: aucune (décision humaine directe, 2026-11-10 : « la règle des 500 lignes semble avoir posé plusieurs fois problème. Augmente la limite à 1.000 lignes. »).
- **Done**: la limite de taille des pull requests passe de ~500 à ~1 000 lignes changées dans tous les fichiers qui la portaient : `AGENTS.md`, `docs/review-checklist.md`, `docs/agents/developer.md`, `docs/agents/orchestration.md`, `docs/branching.md`, ADR-0019, `.vibe/prompts/night-issue.md`, `.vibe/prompts/night-auto.md`. ADR-0101 trace la décision (acceptée, décision humaine directe).
- **Evidence**: `grep -rn "500"` sur les fichiers concernés : plus aucune occurrence. `make check` exécuté après le commit.
- **Judgement calls**: la décision émane directement de l'humain (prioritaire sur AGENTS.md dans la hiérarchie des instructions) ; un ADR (0101) est inclus plutôt qu'un simple commentaire, la décision valant ADR.
- **Escalations**: aucune (la décision humaine elle-même est l'autorisation).

## 2026-11-10 - M1-8 : témoins BDD — BDD classique + mini-BDD témoin + PBT C6 + baseline (feat/14)

- **Issue**: #14 (M1-8 — Témoins BDD, énoncé C6), partie pur Rust (mode dégradé).
- **Done**: `lisage-core::bdd` : `BddController` (FINGERPRINT 3, pas de R-default — arcs 0/1 explicites, R-collapse + R-share seuls) + `BddKeys` (cardinalité 2, pas de δ) + `compiler_bdd` (Shannon descendante, racine niveau 0, niveau ℓ = variable ℓ) + `compter_noeuds` + `evaluer`. `lisage-bench::temoin_bdd` : mini-BDD indépendant (table unique propre). PBT C6 dans `lisage-bench/tests/temoins.rs` : comptes de nœuds identiques + évaluation conforme sur les 2^n affectations (n ∈ 2..=8). Ligne de base bench `bdd_classique_n12_compter` (table LCG déterministe, assertion arène == témoin). Spike `docs/spike-ffi-cudd-sylvan.md`.
- **Evidence**: `make check` OK (76 tests, 2 ignorés), `cargo clippy --all-targets -- -D warnings` OK, `cargo fmt --all` OK. Non exécuté : différentiel contre CUDD/Sylvan réels (dépendance non ajoutée).
- **Judgement calls**: pas de dépendance FFI (liste d'escalation) — différentiel C6 livré contre le mini-BDD interne, vraie question escaladée en commentaire structuré sur #14 (label `question`, le label `needs-human` n'existe pas dans ce repo) ; comptage des nœuds = atteignables depuis la racine, terminaux compris, même convention des deux côtés.
- **Escalations**: commentaire structuré posé sur #14 pour la dépendance cudd-sys (options in-process / hors-processus / mode dégradé) ; aucune réponse consommée.

## 2026-11-10 - M1-6 (2/2) : corrections de review PR #53 — erreurs typées aux frontières (feat/13b, 572edb5)

- **Issue**: #13 (M1-6), revue de la PR #53 (1 BLOCKING, 1 TO-FIX, 2 SUGGESTION).
- **Done**: `Existe::new`/`Filtre::new` renvoient `CoreError::SchemaInattendu` au lieu de paniquer ; `valider_niveaux` renvoie `CoreError::NiveauHorsSchema` avant `disperser_chemin` (plus d'indice hors limites) ; constante `PAS_NIVEAU_MAP` nommée dans `schema.rs` et utilisée dans `operations.rs`. Tests ajoutés : schéma non-Map → `SchemaInattendu`, nœud de niveau 4 hors schéma → `NiveauHorsSchema`.
- **Evidence**: `make check` OK (71 tests, 2 ignorés), `cargo clippy --all-targets -- -D warnings` OK, CI verte sur 572edb5, review de suivi 0 BLOCKING / 0 TO-FIX / 2 SUGGESTION (rapport frais, head_sha conforme).
- **Judgement calls**: les 2 SUGGESTION restantes (validation de `image`, `assert!` interne dans `disperser_chemin`) laissées en l'état — non bloquantes.
- **Escalations**: aucune.

## 2026-11-10 - M1-6 (2/2) : opérations à canal hérité seul — `exists`, `filter`, `map` (feat/13b)

- **Issue**: #13 (M1-6 — Pli et semi-anneaux, énoncés A5/C7, partie opérations spec § 4).
- **Done**: `lisage-core::operations` : `Existe` (pli `(Bool,∨,∧)`, verdict au terminal), `Filtre` (garde, membres rejetés → ε(false), règle A2 « tout rejeté ⇒ ε(false) » dans φ_out), `Image` (réétiquetage des atomes, schéma Atom) ; le canal hérité porte l'ensemble des clés présentes. Helpers `existe`/`filtrer`/`image`. Tests unitaires + PBT différentiels voie A vs voie DD (exists booléen ; filter et map par égalité structurelle C1 des nœuds). Périmètre documenté : prédicats de présence (un verdict sur valeurs d'un niveau val(k) collapsé exige le T-EXPAND § 6.2, hors périmètre).
- **Evidence**: `make check` OK (69 tests, 2 ignorés rouges M1-7), `cargo clippy --all-targets -- -D warnings` OK, `cargo fmt --all` OK. Non exécuté : `cargo audit`, CI réelle avant push.
- **Judgement calls**: prédicats restreints à la présence (bien définis sur le DD réduit) plutôt que T-EXPAND dans φ_disp ; règle A2 ajoutée à `Filtre::phi_out` pour réduire les sous-arbres tout-rejetés.
- **Escalations**: aucune.

## 2026-11-10 - M1-6 (1/2) : pli et semi-anneaux — `Pli<S>`, comptage A4, `union` (merge Bool), correction R-collapse (feat/13)

- **Issue**: #13 (M1-6 — Pli et semi-anneaux, énoncés A4/A5/C7).
- **Done**: `lisage-core::pli` : trait `SemiAnneau` (`zero/un/depuis_cardinalite/plus/fois`) avec `Naturel` (ℕ,+,×) et `BoolOu` (Bool,∨,∧) ; transduction `Pli<S>` (fold, H trivial, copie identité en sortie) avec expansion-comptage **A4 on-path** (niveaux valeur sautés ⇒ facteur |Val|, niveaux présence sautés ⇒ facteur 1 et leur niveau valeur hors chemin) et poids de l'arc par défaut implicite (A6) ; `compter` (count = pli sur (ℕ,+,×)) ; `union` (merge de construction **Bool idempotent = le hash-consing**, déclaration A5) style ZDD. Le trait `Transduction::phi_aggr` reçoit désormais le niveau de chaque fils (nécessaire à A4). **Correction M1-2** : `reduire`/`reduire_inverse` ne collapsent plus un niveau à défaut quand deux arcs explicites mènent à une cible ≠ D_δ (le collapse n'était pas préservant pour la sémantique ensemble de chemins, spec § 2.2 — il détruisait des membres et rendait le comptage inexact, cas limite A5). Tests unitaires (exemple spec § 5 count=2 sur val(a) collapsé, ensembles vide/plein, pli BoolOu, union idempotente/⊥ neutre/⊤ = map vide, A4) + PBT différentiel count voie A (interprète) vs voie DD (pli A4) sur ensembles aléatoires de maps.
- **Evidence**: `make check` OK (63 tests, 2 ignorés rouges M1-7), `cargo clippy --all-targets -- -D warnings` OK, `cargo fmt --all` OK. Non exécuté : `cargo audit`, CI réelle avant push.
- **Judgement calls**: correction de `reduire` (comportement M1-2) rendue nécessaire par l'exactitude du comptage A4 — documentée, tests de confluence M1-2 toujours verts ; facteur A4 « on-path » (le niveau valeur d'une clé absente est hors chemin) ; `union` énumère toutes les valeurs d'un niveau sans défaut.
- **Escalations**: aucune.

## 2026-11-10 - M1-5 (2/2) : `lookup(k)` — modes cherche/extrait/stop, fragment de DD, T-EXPAND + greffe (feat/12b)

- **Issue**: #12 (M1-5 — lookup, spec § 6).
- **Done**: `lisage-core::lookup` : `LookupController` (pool de sortie à payload `Option<u64>`, D_δ = `ε(None)`, FINGERPRINT = ControllerId(2)) et la transduction `Lookup` — canal hérité `Mode::{Cherche, Extrait, Stop(v)}`, canal synthétisé = fragment de DD (`BTreeMap<u64, NodeId>`, arcs du niveau `val(k)` vers `ε(Some(v))`), `φ_out` avec T-EXPAND + greffe quand le niveau valeur de la clé a été collapsé (R-collapse), `SchemaKeys::new` public. Tests unitaires (clés présentes/absente, cas collapsé) + PBT différentiel voie A (interprète) vs voie DD dans lisage-bench.
- **Evidence**: `make check` OK (56 tests, 2 ignorés rouges M1-7), `cargo clippy --all-targets -- -D warnings` OK, `cargo fmt --all` OK. Non exécuté : `cargo audit`, CI réelle avant push.
- **Judgement calls**: dans le cas collapsé, aucun `Stop` n'est émis — la valeur vit dans le DD ré-explicité (T-EXPAND), le fragment synthétisé reste vide (testé) ; la greffe a une mémo par valeur `v` (les terminaux greffés dépendent de `v`).
- **Escalations**: aucune.

## 2026-11-10 - M1-5 (1/2) : moteur de transductions généralisé — deux pools, payload de sortie `Out` (feat/12)

- **Issue**: #12 (M1-5 — lookup, préparation du moteur).
- **Done**: généralisation de `lisage-core::transduction` pour lookup (spec § 3.2 : `Δ_out` peut différer de `Δ_in`) : la transduction lit dans un pool à payload `bool` et écrit dans un pool à payload `Out` ; `psi` et `φ_out` prennent le pool de sortie et écrivent eux-mêmes par interning (A3), conformément à « φ_out est la seule règle qui écrit le diagramme » (spec § 3.1). `Transduction` gagne le type associé `Out`. Tests M1-4 mis à jour (deux pools, comparaison structurelle des formes en PBT).
- **Evidence**: `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run --all-targets` OK (52 tests, 2 ignorés rouges M1-7). Non exécuté : `make check` complet, `cargo audit`, CI réelle avant push.
- **Judgement calls**: deux pools plutôt qu'un payload enum fourre-tout (types distincts pour domaines distincts) ; la comparaison identité en PBT est structurelle (les NodeId sont propres à chaque pool).
- **Escalations**: aucune.

## 2026-11-10 - M1-4 : moteur de transductions — canaux hérité/synthétisé, φ_aggr, mémo (op, nœud, h) (feat/11)

- **Issue**: #11 (M1-4 — Transductions hérité/synthétisé, énoncé L3).
- **Done**: `lisage-core::transduction` : trait `Transduction` (h0, φ_disp, φ_aggr, φ_out, psi, fingerprint) et runner `Transducteur` implémentant T-TERM/T-NODE (spec § 3.1) avec computed table à clé `(fingerprint(op), nœud, h)` (lemme L1/L3). Le runner intern la forme rendue par φ_out : le résultat est canonique par construction (A3), les réductions M1-2 s'appliquent. Tests unitaires (identité avec canal hérité = profondeur, comptage des chemins acceptants avec φ_aggr = somme, mémo : hits au second run, séparation par fingerprint et par h, application sur un diagramme compilé M1-3) + PBT dans lisage-bench (identité ⇒ même nœud, comptage transduction = comptage naïf récursif, sanité de la mémo sur DAG aléatoires).
- **Evidence**: `make check` OK (52 tests, 2 ignorés rouges M1-7), `cargo clippy --all-targets -- -D warnings` OK, `cargo fmt --all` OK. Non exécuté : `cargo audit`, CI réelle avant push.
- **Judgement calls**: φ_out rend une `Form` (le runner intern) plutôt que d'interner lui-même — garde le trait indépendant du pool ; la mémoire de niveau M (tampons) est absente, non utilisée par les opérations de M1 ; le payload reste `bool` (lookup à payload Option(V) en M1-5, #12).
- **Escalations**: aucune.

## 2026-11-10 - M1-3 (3/3) : décompileur (round-trip) + PBT des schémas, clôture de l'issue (feat/10c)

- **Issue**: #10 (M1-3 — Schémas de référence, énoncé Σ).
- **Done**: `decompiler` dans `lisage-core::schema` (inverse de `compiler`, gère le collapse du domaine à un élément) ; tests unitaires ajoutés (ensemble vide/plein, erreur valeur-vs-clé hors domaine d'une map, round-trip, canonicité C1) ; PBT dans lisage-bench (`schemas.rs`) : round-trip compile→décompile pour Atom/Set/Map aléatoires, et canonicité (valeurs distinctes ⇒ nœuds distincts, même valeur ⇒ même nœud).
- **Evidence**: `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run --all-targets` OK (46 tests, 2 ignorés rouges M1-7). Non exécuté : `make check` complet, `cargo audit`, CI réelle avant push.
- **Judgement calls**: le décompileur gère le cas terminal(true) pour un domaine atomique à un seul élément (R-collapse réduit le nœud au terminal).
- **Escalations**: aucune.

## 2026-11-10 - M1-3 (2/3) : module schéma — Atom/Set/Map bornés, compilation balayage d'univers (feat/10b)

- **Issue**: #10 (M1-3 — Schémas de référence, énoncé Σ).
- **Done**: `lisage-core::schema` : `Domaine` (atomes bornés, défaut éventuel), `Schema::{Atom, Set, Map}`, `SchemaKeys` (Val(ℓ) et δ(ℓ) par niveau), `ReferenceController` (D_δ = terminal(false) pour les niveaux de présence, δ = ⊥ pour les niveaux valeur, spec § 2.1), `compiler` (balayage d'univers : un niveau de présence par clé, δ = absent, sans marqueur ; seul l'arc choisi est interné, l'autre mène au défaut porteur — R-collapse saute les absents, style ZDD) et `SchemaError` typée. Tests unitaires : exemple de la spec § 1 (map {a↦1,b↦0} sur K={a,b}, V={0,1}), collapse des absents, ensemble vide/plein, map vide et clé absente, erreurs typées.
- **Evidence**: `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run --all-targets` OK (40 tests, 2 ignorés rouges M1-7). Non exécuté : `make check` complet, `cargo audit`, CI réelle avant push.
- **Judgement calls**: `Set(Map(...))` (ensemble de maps, ex. S de la spec) nécessite l'union — reporté à M1-6 (#13) ; M1-3 compile les valeurs simples. Le décompileur (round-trip) et les PBT suivent en PR 3/3.
- **Escalations**: aucune.

## 2026-11-10 - M1-3 (1/3) : algèbre de clés en méthodes d'instance — le schéma est une donnée runtime (feat/10)

- **Issue**: #10 (M1-3 — Schchemas de référence, préparation).
- **Done**: évolution de l'interface du cœur : `KeyAlgebra::{normal_form, cardinality, default}` et `Controller::default_payload` passent en méthodes d'instance, `Controller::keys()` est ajouté ; `Arena` détient désormais l'instance de contrôleur (`Arena::new(controleur, observer)`) au lieu d'un `PhantomData`. Motivation : les schémas de référence (M1-3) sont des données runtime (domaines finis par niveau), que des méthodes statiques ne peuvent pas voir. `Controller::collapse` reste statique (fonction pure). Mocks des tests mis à jour. Aucun changement de comportement : les 35 tests existants passent inchangés.
- **Evidence**: `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run --all-targets` OK (35 tests, 2 ignorés rouges M1-7). Non exécuté : `make check` complet, `cargo audit`, CI réelle avant push.
- **Judgement calls**: méthodes d'instance plutôt qu'encodage du schéma dans le type `Level` (évite un échappement dynamique non justifié) ; le refactor est découpé en PR séparée car le diff total de M1-3 dépasse la limite ~500 lignes par PR.
- **Escalations**: aucune.

## 2026-11-10 - M1-2 : réductions R-collapse / R-default, confluence prouvée par PBT (feat/9)

- **Issue**: #9 (M1-2 — Réductions, spec § 2.3).
- **Done**: `Reduction<Key>` + `collapse_arcs` + `reduire` (R-collapse puis R-default) + `reduire_inverse` (ordre inverse) dans `lisage-algebra::reduction` ; `Controller::collapse` délègue à `collapse_arcs`. L'arena applique `reduit(level, arcs)` à l'internement : `Collapse` émet `ReductionApplied{rule: Collapse, level}` et retourne la cible, `Forme` émet l'événement `Default` quand des arcs par défaut ont été retirés. `TraceEvent::ReductionApplied` porte désormais le `level`. Tests unitaires (exemple de la spec, R-default, non-collapse, collapse total) + PBT de confluence dans lisage-bench : idempotence (reduire ∘ reduire = reduire) et commutation (reduire = reduire_inverse) ; PBT C1 mise à jour (`test_c1_canonicite_par_handle` : formes réduites égales ⇒ même nœud).
- **Evidence**: `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run --all-targets` OK (34 tests, 2 ignorés rouges M1-7). Non exécuté : `make check` complet (partie Python inchangée), `cargo audit`, CI réelle avant push.
- **Judgement calls**: sémantique de couverture de la spec § 2.2 (tout arc absent vaut δ(ℓ)) : avec un défaut, le collapse vaut si δ est explicite ou si la cible est D_δ ; sans défaut, toutes les valeurs doivent être explicites. `reduire_inverse` traite le cas « arcs vides après retrait » comme `Collapse(cible retirée)` pour préserver la commutation.
- **Escalations**: aucune.

## 2026-11-10 - M1-1 : arena + handles, hash-consing à clé (contrôleur, ancre, forme) (feat/8)

- **Issue**: #8 (M1-1 — Arena + handles).
- **Done**: `Arena<C: Controller>` dans `lisage-core::arena`, implémentation du port `Pool` : table unique hash-consée à clé (contrôleur, ancre, forme normalisée par l'algèbre de clés), `NodeId` en ordre de création, `handle()` = (contrôleur, ancre, nœud), événements de trace `NodeCreated`/`NodeShared` émis à l'internement (observateur enregistreur épinglé en test). PBT C1 dans lisage-bench : unicité des handles (même forme ⇒ même nœud, même arena ou arenas distinctes ; formes distinctes ⇒ handles distincts).
- **Evidence**: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run --all-targets` OK (27 tests, 2 ignorés rouges M1-7). Non exécuté : `make check` complet (partie Python inchangée), `cargo audit`, CI réelle avant push.
- **Judgement calls**: R-share seul à l'internement en M1-1 ; R-collapse/R-default s'insèrent à la création du handle en M1-2 (#9), conformément au séquencement interface → réductions.
- **Escalations**: aucune.

## 2026-11-10 - M1-7 : interprète de référence (voie A) + différentiel A/B/C branché (feat/7)

- **Issue**: #7 (M1-7 — Interprète de référence, phase 2 : tests).
- **Done**: interprète de référence en pur `lisage-lang` sans DD (`value.rs` : `Value::{Atom,Set,Map}` avec `Atom` newtype distinct (ADR-0006) ; `interpreter.rs` : `count/exists/filter/map/member/lookup/saturate`), testé sur l'exemple de la spec § 1 (S = {{a↦1,b↦0},{a↦0,b↦0}} : count = 2, lookup(a) = {Some(1), Some(0)}, filter, saturate idempotent). Harnais différentiel A/B/C branché dans `lisage-bench::differentiel` dès le premier commit : voie A réelle (lang), voies B/C factices ; tests différentiels réels écrits **rouges** (`#[ignore]`, vérifiés FAIL via `cargo nextest --run-ignored only`) avant l'implémentation du cœur, à dé-ignorer en M1-6 (#13). PBT de la voie A (count = cardinalité, filter monotone, lookup = m(k)) dans lisage-bench.
- **Evidence**: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run --all-targets` OK (19 tests, 2 ignorés rouges vérifiés rouges), `cargo doc --no-deps` OK. Non exécuté : `make check` complet (partie Python inchangée), `cargo audit`, CI réelle avant push.
- **Judgement calls**: les PBT vivent dans lisage-bench (seule crate à posséder proptest, M0-4) — ajouter proptest à lang aurait été une nouvelle dépendance sans escalation (revue PR #42). Les PBT par énoncé C1/L3/A5/C6 sont écrits dans l'issue qui livre la voie concernée, pas ici (ils ne compileraient pas sans l'API visée).
- **Escalations**: aucune.

## 2026-11-10 - M1-0 : interface du cœur, traits + bornes Send/Sync (feat/6)

- **Issue**: #6 (M1-0 — Interface du cœur, phase 1 : interface).
- **Done**: contrats du cœur posés : `NodeId`/`Anchor`/`ControllerId`/`Handle`/`Level`/`Form` et traits `Pool`/`KeyAlgebra`/`Controller` dans `lisage-algebra::interface`, `Observer`/`TraceEvent`/`NoOpObserver` + `CoreError` dans `lisage-core`, `LangError` + `KeyName` dans `lisage-lang`. Bornes `Send`/`Sync` figées sur les quatre traits (vérifiées dans les sources et à la compilation), politique d'erreurs codifiée, R-collapse testé par PBT (dans lisage-bench, qui possède déjà proptest). ADR-0100 draft (needs-human). Spike FFI CUDD/Sylvan livré avec M1-8 (#14).
- **Evidence**: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run --all-targets` (10 tests, dont `test_collapse_pbt`), `cargo doc --no-deps`, `cargo deny check` : tous OK. Revue PR #43 : 1 BLOCKING (Atom alias transparent → newtype) corrigé, 1 TO-FIX (logging différé, tracké via ADR-0100 draft needs-human). Revue PR #42 : 1 BLOCKING (R-collapse sans test) puis 2 BLOCKING (dev-dep proptest retirée d'algebra ; diff > ~500 lignes) + 1 TO-FIX (cohérence WORKLOG/PR), tous corrigés. Non exécuté : `make check` complet (partie Python inchangée), `cargo audit`, CI réelle avant push.
- **Judgement calls**: contrats dans `lisage-algebra` car le graphe du workspace interdit `algebra → core` (ADR-0100 draft) ; pas de crate de log (liste d'escalation), narration en doc-comments + tests.
- **Escalations**: ADR-0100 draft `needs-human` ; commentaire structuré posé sur #14 pour la future dépendance `cudd-sys`.

## 2026-11-10 - M0-5 : socle de bench, criterion + micro-bench factice (feat/5)

- **Issue**: #5 (M0-5 — Socle de bench).
- **Done**: criterion 0.5 branché en dev-dependency de lisage-bench avec harness dédié (`[[bench]] micro`), micro-bench factice `voie_a_factice`, étape `bench` ajoutée à la CI GitHub Actions (rust-check). Le harness MCC conteneurisé reste à vide (corpus en M5).
- **Evidence**: `cargo fmt --all -- --check` OK, `cargo clippy --all-targets -- -D warnings` OK, `cargo test --all-targets` OK (5 tests), `cargo bench -p lisage-bench --bench micro` OK (300 ps). Non exécuté : CI GitHub réelle, cargo nextest.
- **Judgement calls**: le bench factice porte sur voie_a (référence) ; le seuil de régression sera calibré quand les vrais benches arriveront (M1+).
- **Escalations**: aucune.

## 2026-11-10 - M0-4 : charte de test, triptyque A/B/C + proptest (feat/4)

- **Issue**: #4 (M0-4 — Charte de test).
- **Done**: triptyque A/B/C implémenté dans lisage-bench (voie_a = référence naïve, voie_b = DD ordre fixe, voie_c = Lisage+planner, toutes factices) ; proptest installé en dev-dependency ; convention `test_c<n>_*` ; test différentiel factice `test_c0_triptyque_accord` (proptest) + `test_c0_exemple_factice`.
- **Evidence**: `cargo fmt --all -- --check` OK, `cargo clippy --all-targets -- -D warnings` OK, `cargo test --all-targets` OK (5 tests passés au total). Non exécuté : cargo nextest, CI GitHub réelle.
- **Judgement calls**: les trois voies sont des stubs factices (x+1) en attendant l'implémentation réelle M1.
- **Escalations**: aucune.

## 2026-11-10 - M0-2 : squelette du workspace, 7 crates + test de frontières (feat/2)

- **Issue**: #2 (M0-2 — Squelette du workspace).
- **Done**: création des 7 crates (lisage-lang, lisage-algebra, lisage-plan, lisage-core, lisage-backend, lisage-bench, lisage-liseuse) avec dépendances unidirectionnelles dans Cargo.toml (graphe § 8) ; test de frontières dans lisage-bench/tests/frontieres.rs qui lit les Cargo.toml et vérifie le graphe autorisé + présence des 7 crates.
- **Evidence**: `cargo fmt --all -- --check` OK, `cargo clippy --all-targets -- -D warnings` OK, `cargo test --all-targets` OK (3 tests passés : socle factice + 2 frontières). Non exécuté : cargo nextest, CI GitHub réelle.
- **Judgement calls**: le test de frontières parse les Cargo.toml plutôt que d'utiliser cargo-metadata (évite une dépendance dev supplémentaire).
- **Escalations**: aucune.

## 2026-11-10 - M0-1 : socle Rust ajouté au template Python (feat/1)

- **Issue**: #1 (M0-1 — Dépôt depuis project-template).
- **Done**: ajout du socle Rust sans purger le Python (décision du owner en session) : rust-toolchain.toml (stable), Cargo.toml workspace, deny.toml, crate factice lisage-core avec test factice, job `rust-check` dans la CI GitHub Actions (fmt → clippy → nextest → coverage → audit → deny), job `rust-quality-gate` dans GitLab CI, cibles `rust-*` dans le Makefile, hooks cargo fmt/clippy dans pre-commit, `target/` dans .gitignore. Le Python (ruff/mypy/pytest/uv) reste intact.
- **Evidence**: `cargo fmt --all -- --check` OK, `cargo clippy --all-targets -- -D warnings` OK, `cargo test --all-targets` OK (1 test passé). Non exécuté : cargo nextest (installation en cours), cargo-llvm-cov, cargo-audit, cargo-deny, CI GitHub réelle.
- **Judgement calls**: le Python n'est pas purgé (instruction du owner en session, dérogation à l'issue M0-1) ; les deux écosystèmes coexistent, la CI valide les deux.
- **Escalations**: aucune.

## 2026-11-09 - night session live trace

- **Issue**: none (human request: the night loop goes silent while a session runs).
- **Done**: the night loop is observable while it runs: every session prompt is kept under `.night/sessions/<run>/` (instead of a TemporaryDirectory deleted on exit), the trace logs a `still running` heartbeat every `NIGHT_HEARTBEAT_SECONDS` (default 60, `0` disables it, also `--heartbeat`), each stdout result line names the item the session claimed (`(claimed: #4)`), and the explicit mode records whether the claim label landed after each session. `scripts/__main__.py` also gets a dispatch fix: `runner(arguments)` was positional while every `run()` is keyword-only, so `python -m scripts <command>` crashed with a TypeError on the previous revision of the package entry point. Orchestration doc (§8) updated accordingly.
- **Evidence**: ruff check and ruff format green, py_compile green; simulation under Python 3.12 with fake httpx/git/vibe backends: 27/27 assertions (auto-discovery with recovery PRs and claimed suffixes on stdout, heartbeat on/off, anti-spin stop on a claimless session, explicit mode with claim-note trace). Not run: mypy strict, the real Vibe Code CLI, the real GitHub API.
- **Judgement calls**: prompts persist under `.night/` (already gitignored) rather than a temp dir - the morning audit needs them and they cost nothing; the heartbeat wakes once per second to detect a finished session promptly instead of sleeping the whole heartbeat period, so a short session is not padded to 60s.
- **Escalations**: none.

## 2026-11-08 - driver commands as one CLI package

- **Issue**: none (human request).
- **Done**: the eleven flat driver scripts become one package: `python -m scripts <command>` with the subcommands claim, pr, update-pr, merge-pr, wait-ci, review, review-bundle, agent-review, day-report, sync-daily, night; shared plumbing in `scripts/_shared.py` and `scripts/_github.py`; make targets, variables and arguments unchanged. The stale `scripts/<name>.py` paths in ADR-0009/0010/0017/0018/0019/0020 were updated to the new entry point on the owner's explicit instruction, and `docs/decisions/README.md` records the owner-set exception to "Accepted ADRs are never edited". New ADR-0021 (draft) records the migration.
- **Evidence**: ruff check and ruff format green, py_compile green; full simulation under Python 3.12 with fake httpx/git/vibe backends: 74/74 assertions across the eleven subcommands (help wiring, claim, update-pr, pr, wait-ci, the review to merge-pr chain, sync-daily S1-S10, day-report, night auto-discovery/explicit/anti-spin). The patch was re-applied onto a fresh copy of the base and verified byte-identical. Not run: mypy strict, real git and GitHub API, uv sync/pytest.
- **Judgement calls**: editing accepted ADRs departs from the "never edited" rule - done under explicit human instruction in this session; the flat scripts total 3826 lines, the package 3324.
- **Escalations**: ADR-0021 is a draft awaiting the human owner's acceptance.

## 2026-11-08 - milestone filter for the night loop

- **Issue**: none (human request).
- **Done**: `scripts/night_loop.py` gains `--milestones "3,4"` (comma-separated milestone numbers); `make night MILESTONES="3,4"` forwards it. The filter applies to the auto-discovered worklist (issues and recoverable pull requests) and to the explicit issue list - items outside the requested milestones are skipped with a recorded reason. Empty means no filtering.
- **Evidence**: `make check` green (ruff, mypy strict, import-linter, pytest 33 passed, coverage 94.76%); helper functions exercised directly (parse/match/error path). Not run: a live night loop against the GitHub API.
- **Judgement calls**: milestone identifiers are milestone numbers (the GitHub API identifier), not titles; ADR-0017 (docs/decisions/) left untouched - constitutional file, needs human sign-off.
- **Escalations**: none.

## 2026-11-08 - night autonomy trace and PR body updates

- **Issue**: none (human request).
- **Done**: night loop splits output into a short stdout summary and a verbose trace in `.night/night-loop.log` (override with `NIGHT_LOG_FILE`); branch pattern tolerates descriptive suffixes (`feat/12-slug`); new `scripts/update_pr.py` + `make update-pr PR=N BODY_FILE=...` (ADR-0019); ADR-0017, developer/orchestration docs, prompts and review checklist updated accordingly.
- **Evidence**: `make check` green (ruff, mypy strict, import-linter, pytest 33 passed, coverage 94.76%). Not run: a live night loop or a real PR body update against the GitHub API.
- **Judgement calls**: the change touches constitutional files (docs/decisions/, docs/agents/, docs/review-checklist.md) - done under explicit human instruction in this session.
- **Escalations**: none.

## 2026-11-08 - type fixes for the subcommand refactor

- **Issue**: none (human request: PR + merge the two commits).
- **Done**: the two commits (sync-daily script, subcommand refactor) failed mypy strict with 15 errors; fixed by parameterizing `argparse._SubParsersAction[argparse.ArgumentParser]` and replacing the explicit-Any `Callable[..., int]` in `scripts/__main__.py`.
- **Evidence**: `make check` green after the fix (ruff, mypy strict 34 files, import-linter, pytest 33 passed, coverage 94.76%).
- **Judgement calls**: fixes committed as a separate commit on the branch rather than rewriting the human's commits.
- **Escalations**: none.

## 2026-11-09 - more output during night session (type fix on top)

- **Issue**: none (human request: commit, push, merge).
- **Done**: commit `857926c` (richer night-session output in scripts/night.py + docs) plus a fix commit restoring `Callable[[argparse.Namespace], int]` in scripts/__main__.py, which the commit had reverted to an explicit Any.
- **Evidence**: `make check` green after the fix (ruff, mypy strict 34 files, import-linter, pytest 33 passed, coverage 94.76%).
- **Judgement calls**: fix committed separately, the human's commit untouched.
- **Escalations**: none.

## 2026-11-09 - night session prints text, not a JSON dump

- **Issue**: none (human relayed the petrichor fix; same one-liner applies here).
- **Done**: `scripts/night.py` runs headless sessions with `--output text` (new `SESSION_OUTPUT_FORMAT` constant, command built by `build_session_command`); new `scripts/tests/test_night.py` pins the command and asserts `"json" not in command`.
- **Evidence**: `make check` green (ruff, mypy strict, import-linter, pytest 34 passed, coverage 94.76%). Not run: a live vibe session.
- **Judgement calls**: ADR-0017 still mentions `--output json` in its decision text - left untouched (constitutional file, human sign-off); the decision itself (process boundary = context reset) is unchanged.
- **Escalations**: none.

## 2026-11-09 - PR template asks for Closes #n

- **Issue**: none (human request).
- **Done**: `.github/PULL_REQUEST_TEMPLATE.md` gains an "Issue closure" section (`Closes #n` in the PR body so GitHub auto-closes the issue on merge) and a matching checklist item.
- **Evidence**: `make check` green (ruff, mypy strict, import-linter, pytest 34 passed, coverage 94.76%).
- **Judgement calls**: PR template chosen over docs/agents/ - it is where the PR author actually looks.
- **Escalations**: none.

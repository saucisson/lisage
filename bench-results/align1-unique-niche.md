# ALIGN-1 — NonZero<u32> hash tag in the unique-table slot

Machine: M2

```json
{"machine_id": "M2", "hardware_hash": null, "note": "M2 fingerprint not captured while this session ran on M2 (2026-11-20 morning); fields unverified, requested from the human — see bench-results/machines.md. The hash febd14ed is M1, captured on 2026-11-20 evening after the switch back."}
```


Runner `profile_intern {n}`, one process per run, counting global allocator (MEM-18 pattern).
Same charge as the PERF publications: alea-lcg, bottom-up Shannon expansion, 2^n - 1 intern calls.

Reproduce the after-row (median of 3 direct runs, this machine, 2026-11-20):

```
cargo run --release -p lisage-bench --bin profile_intern   # n=20 default
ALIGN1_PUBLISH=1 cargo nextest run -p lisage-bench --test profile_intern   # republish hook
```

Note: under the nextest harness the runner is ~10x slower (test-process contention on this
4-core WSL2 box); the direct `cargo run --release` numbers are the reference.

## Before / after (n=20, same machine, same runner, same session)

| row | noeuds | internements | temps table (ms) | temps build (ms) | temps count (ms) | allocs build | octets build | allocs/internement | rss_max_ko |
|---|---|---|---|---|---|---|---|---|---|
| before (daily @ 9876585, Option<Slot> = 16) | 106760 | 1048575 | 1.187 | 69.818 | 14.087 | 1048803 | 47256948 | 1.000 | 13432 |
| after (ALIGN-1, Option<Slot> = 12; median of 3 direct runs) | 106760 | 1048575 | 1.165 | 69.062 | 14.789 | 1048803 | 45159828 | 1.000 | 12296 |

Node and internement counts are identical (C1, differential green); allocs/internement stays
at the 1.000 floor; build time unchanged within noise (69.8 -> 69.1 ms median).

## What changed

`Slot.hash` is now `NonZero<u32>` (`crates/lisage-core/src/unique.rs`); a truncated hash of 0
is remapped to 1 by `slot_tag` (documented invariant: the tag only steers bucket placement,
equality is always confirmed semantically through the stored node's form). The zero niche
makes `Option<Slot>` 12 bytes instead of 16 (tag + padding before) — pinned by
`test_align1_option_slot_is_12_bytes`, plus `test_align1_slot_tag_remaps_zero` for the remap.
`TableUnique::bytes()` follows automatically (`capacity * size_of::<Option<Slot>>()`).
`hash_cle` and its golden value are untouched.

## Memory effect (the hypothesis, falsified or confirmed)

Hypothesis: ~-8 bytes/node on the unique table at load factor 1/2 (32 -> 24 bytes/node).
Measured:

- alea-lcg-20: the table holds 106 760 keys at capacity 2^18 = 262 144 slots; 4 bytes/slot
saved = 1 048 576 bytes end-to-end (~9.8 bytes/node at this load factor ~0.41). Transient
`octets_build` drops 47 256 948 -> 45 159 828 (-2 097 120 bytes, the sum over the
doubling growths, ~2 x 1 048 576); RSS max 13 432 -> 12 296 KiB (-8.5%).
- MEM-19 charge (20 002 nodes, capacity 2^16 = 65 536 slots): end-to-end bytes/node
80.42 -> 67.32 (-13.10 = 65 536 slots x 4 bytes / 20 002 nodes);
`bench-results/mem19-layout-on-off.md` republished via `MEM19_PUBLISH=1` (the test's
designed update path), node counts identical.

At load factor exactly 1/2 the saving is 8 bytes/node as hypothesized; at the actual load
factors of these charges it is 9.8-13 bytes/node. Hypothesis confirmed in direction and
order of magnitude.

## Gates

- C1: node/internement counts identical; `differentiel` tests green (5 passed, 2 skipped —
the skipped ones need the external witness binaries).
- MEM-18 thresholds: green (`make check`, 369 passed / 11 skipped on this commit).
- allocs/internement: 1.000 unchanged (the slot niche touches no allocation).

## Verdict (ALIGN-7e) — the PERF-5 vs ALIGN-1 "before" gap is the machine, not the code

The PERF-5 publication (`perf5-shadow-index.md`, **Machine: M1**) reports 132.1 ms build
time at n=20; the ALIGN-1 "before" row above (**Machine: M2**) reports 69.818 ms on the
same charge, same runner, same session protocol — with byte-identical allocation counters
(1 048 803 allocs build, 1.000 alloc/internement). The work performed is therefore
identical and the 132.1 -> 69.8 ms gap is attributed to the **M1 -> M2 machine change**
of the evening of 2026-11-19 (see `bench-results/machines.md`), not to any code change.

**Reformulated nextest note**: the note above ("under the nextest harness the runner is
~10x slower") remains true in itself for test-suite runs (nextest runs test binaries in
parallel across cores, unlike a single-process runner), but it does **not** explain this
gap — both rows come from the same single-process `profile_intern` runner, so the factor
between them is the hardware, not the test harness.

**Consequence**: absolute times of the PERF series (M1) and the ALIGN series (M2) are
**not comparable to each other**; only ratios within one campaign on one machine are.
The lisage/OxiDD ratio of campaign-3 (2.04x at n=24) remains exact: the Docker witnesses
share the host, so a machine change applies to all four tools at once and inter-tool
ratios stay valid (ALIGN-7c).

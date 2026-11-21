# ALIGN-2 — key elided from the dense-family slots (`Option<(Key, NodeId)>` → `Option<NodeId>`)

Charge that activates the dense family: the MEM-19 layout runner in `off` mode
(`cargo run --release -p lisage-bench --bin layout -- off 20000`) — BDD nodes of
degree 2 with `layout_inline` off go through `choose_dense_family` (cardinality 2,
fill 2/2 ≥ 1/3 → dense). The main `profile_intern` charge (layout on) stores inline
exact entries and is measured as a control.

Reproduce (this machine, 2026-11-20, median of 3 runs each):

```
cargo run --release -p lisage-bench --bin layout -- off 20000   # dense charge
cargo run --release -p lisage-bench --bin layout -- on 20000    # inline control
cargo run --release -p lisage-bench --bin profile_intern        # alea-lcg-20 control
```

## Before / after (n=20000, same machine, same session; before = daily @ 53c86f2)

| row | nodes | bytes/node (end-to-end) | arena bytes/node | temps_ms (median) | rss_max_ko (median) |
|---|---|---|---|---|---|
| off before (dense slot `Option<(u64, NodeId)>` = 24 B) | 20002 | 83.32 | 40.00 | 4.585 | 7324 |
| off after (dense slot `Option<NodeId>` = 8 B) | 20002 | 75.32 | 32.00 | 4.636 | 7704 |
| on before = on after (inline control, unchanged) | 20002 | 67.32 | 24.00 | 3.762 | 5704 |

Node counts identical on/off and before/after (C1). `bench-results/mem19-layout-on-off.md`
republished via `MEM19_PUBLISH=1`; the layout test's arena-factor range re-pinned on this
measured baseline (1.67x → 1.33x = 32.00/24.00), per the frozen-range convention.

Control, alea-lcg-20 (`profile_intern`, median of 3): nodes 106 760, internements
1 048 575, allocs/internement **1.000 unchanged**, octets_build 45 159 828 → 45 159 844
(+16 B = the one shared per-level enumeration table), rss_max ~12 408 KiB, build ~74 ms
(machine noise; the charge stores inline entries, dense not chosen).

## What changed

- `ArcRun::Dense` now wraps a boxed `DenseRun<Key> { keys: Arc<[Key]>, slots: Box<[Option<NodeId>]> }`
  (`crates/lisage-algebra/src/interface.rs`). By ordinal construction the key of slot `r`
  is `keys[r]`, re-emitted by rank from the shared per-level enumeration — the key is no
  longer stored per slot. The `ArcStore` ordered-iteration contract is unchanged
  (`iter()` still yields `(&Key, &NodeId)`, in rank order, same pairs); the family choice
  in `choose_dense_family` is unchanged and deterministic (same threshold
  `4·|Val(ℓ)| ≤ 12·arcs`).
- `KeyAlgebra::enumeration(level) -> Arc<[Key]>` (new provided method, default built from
  `unrank`); `BddKeys` and `SchemaKeys` override it with a cached table so the dense path
  clones one `Arc` per node — **no per-node allocation** (allocs/intern stays 1.000).
- Side effect measured: the `ArcRun<u64>` enum went 24 → 16 bytes (the `Sorted` fat
  pointer no longer sets the enum size once `Dense` is a thin box), so `NodeEntry`
  went 40 → 32 bytes — that is the −8 bytes/node of the MEM-14 arena counter.
- Heap per dense node (u64 keys, cardinality 2): slots 2 × 24 = 48 B → 2 × 8 = 16 B
  (+16 B `DenseRun` box header) = 32 B, i.e. −16 B/node real, plus one shared 16 B
  table per level. Witnesses: `test_align2_dense_slot_is_8_bytes` (plus the ALIGN-0
  premise `Option<(u32, NodeId)>` = 12 kept).
- Contract note: the dense family now also requires `unrank` (via `enumeration`) —
  documented on `KeyAlgebra::rank`; an algebra with `rank` but no `unrank` stays on
  the sorted family (the `Trois` test mock gained `unrank`).

## Gates

- C1: node counts identical everywhere; `dense` tests green (lisage-core 5/5,
  lisage-bench 4/4 incl. `test_c1_famille_dense`); PBT `dense.rs` green.
- MEM-18 thresholds green; allocs/internement 1.000 on the reference charge.
- `make check` green on the commit (370 passed, 11 skipped).

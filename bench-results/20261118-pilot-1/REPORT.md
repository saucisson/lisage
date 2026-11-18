# Pilot report — run 20261118-pilot-1 (MEM-7, issue #123)

Pilot measurement of the MEM-A port (ArcStore/ArcRun layout), the
generalization gate: **gap = constant factor → the layout is validated
and MEM-B opens; complexity gap → stop and escalate.** Every number
below comes from the deposited files of this directory
(`aggregats.json`, `<charge>-<outil>.jsonl`); nothing is unsourced.

- Machine: local developer machine (WSL2), 2026-11-18.
- Repetitions: 5 per charge x tool, one fresh process per measurement
  (fresh Docker container for OxiDD).
- Reproduction: see `README.md` in this directory.
- OxiDD pinned at commit `c7a458df8b747a3ee44d0e9e61059a1621865fcd`.

## Cross-check (validity of the run)

`noeuds` and `verifie` are identical across tools for `alea-lcg-20`
(the orchestrator's divergence list is empty). The lisage-only lines
match the pinned semantic values of `tests/confrontation.rs`:
|R_5| = 152, window 4x10 = 40 paths with 12 nodes (absolute) vs 3
(relative).

| Charge | noeuds | verifie |
|---|---|---|
| alea-lcg-20 (lisage = OxiDD) | 106760 | 523228 |
| philosophers-n5 (lisage) | 73 | 152 |
| window-4x10-absolu (lisage) | 12 | 40 |
| window-4x10-relatif (lisage) | 3 | 40 |

## Time (ms, median [min, max])

| Charge | lisage | OxiDD | ratio lisage/OxiDD |
|---|---|---|---|
| alea-lcg-20 | 537.9 [532.2, 550.8] | 129.8 [128.9, 132.0] | 4.1x |
| philosophers-n5 | 3.0 [2.9, 3.1] | — | — |
| window-4x10-absolu | 0.026 [0.025, 0.028] | — | — |
| window-4x10-relatif | 0.015 [0.013, 0.017] | — | — |

## Peak RSS (kio, median [min, max])

| Charge | lisage | OxiDD | ratio OxiDD/lisage |
|---|---|---|---|
| alea-lcg-20 | 31000 [30972, 31016] | 50328 [50172, 50424] | 1.6x |
| philosophers-n5 | 2852 [2832, 2876] | — | — |
| window-4x10-absolu | 2296 [2292, 2312] | — | — |
| window-4x10-relatif | 2300 [2268, 2312] | — | — |

## Decision (written, per issue #123)

- **Node counts**: identical across tools where a witness exists
  (alea-lcg-20: 106760 = 106760, C6 cross-tool), and equal to the
  pinned theoretical values on the lisage-only lines (|R_5| = 152,
  window 12 vs 3 nodes). No complexity gap in the diagram sizes.
- **Time**: lisage is a constant factor ~4.1x slower than OxiDD on the
  pilot BDD charge — the same constant factor as the M7-6 campaign on
  the real charges (3.5x-4.0x), not a growing one.
- **Memory**: lisage uses a constant factor ~1.6x less peak RSS than
  OxiDD on the pilot charge (OxiDD pre-allocates its unique table and
  apply cache at 2^20 entries each — witness sizing constant).

**Gap = constant factor on every pilot line → the layout is
validated: MEM-B (arenas per class) opens.** No complexity gap was
observed, so no stop-and-escalate. Falsifiable: the run is reproducible
with the exact command of `README.md` (deterministic LCG seed, versioned
states file, pinned cross-check values).

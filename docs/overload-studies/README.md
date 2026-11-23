# Overload studies, parts I to VII

Archive of the seven experimental studies devoted to the question: **can we do
better with overload annotations on lisage decision diagrams?** applied to
model families spotted in the Model Checking Contest catalogue
(Diffusion2D-like, Referendum, NQueens). Kept in the repository
(`docs/overload-studies/`, issue ARCH-1): each study is reproduced by its
bench, and the JSON results are regenerated identically (deterministic
benches).

## Contents

```
studies/   01..07: the seven studies, exact copies of the published documents
benches/   study5.py, study6.py, study7.py: Python benches (parts V, VI, VII)
benches/   results5.json, results6.json, results7.json: regenerated results
```

- `studies/`: the files carry their original YAML header (`name`, `title`,
  `type`); this is document metadata, with no effect on the content.
- The benches write their results next to themselves (relative `os.path`):
  they run as-is in any repository folder.
- The results of parts I to IV are not regenerated here: their historical
  benches (study1-4.py) were not rebuilt, only the numerical results appear
  in studies 01 to 04.

## The laws of the series, one line each

1. **Anchoring alone never suffices** (I): the flat/pool gap plays out on
   the window and the seams, not on the choice of anchor.
2. **The gap explodes on globally filtered languages** (II): petN ×49 at n=6,
   polynomial pool (556) against a super-linear flat (27 462).
3. **Valued edges and clusters cash in the workflow density** (III-IV): the
   Θ(K²) of the FMS was a scheduling artefact, not a fatality.
4. **The seam, not the symmetry, carries the gain** (V): the pressed torus
   keeps the torus flat size but triples its interior classes.
5. **When the guard or the query is global, the bounded window is not
   expensive, it is wrong** (VI): 20 to 40 % false positives on N-queens,
   incorrect types from N=7 on the referendum without multiplicities.
6. **The span of the guards in the variable order decides everything, and
   the controller can read it statically** (VII): every flat node is a cut
   memory motif; perfect equality on 328/328 levels, the classifier recovers
   five historical verdicts out of five.

## Key tables

### V: 2D n×n grids, complete series (torus, cylinder, plane, pressed torus)

| variant | flat n=2..5 | pool n=2..5 | A/B at n=5 |
|---|---|---|---|
| torus | 8 / 39 / 225 / 1159 | 11 / 43 / 158 / 300 | 17 / 283 |
| cylinder | 8 / 43 / 241 / 1407 | 13 / 52 / 174 / 299 | 17 / 282 |
| plane | 8 / 32 / 95 / 249 | 22 / 107 / 277 / 519 | 34 / 485 |
| pressed torus | 8 / 39 / 225 / 1159 | 29 / 143 / 535 / 978 | 51 / 927 |

The pressed-torus flat is identical to the torus (the overload letter at
(0,0) creates no additional node); its interior pool triples (3×17): the
pressing cashes in classes, not space. |S| n=6 by transfer matrix (printed
at the start of study5.py): torus 2 406 862, cylinder 3 557 711.

### VI: memory (extremes)

| family | breakdown |
|---|---|
| referendum N=10 | \|S\|=59 049, flat=10, flat+query=51, counter=66, multiset pool 3 945, set pool 3 563, 205 ambiguous classes of which 34 decision-ambiguous, 5 count values in one class |
| N-queens n=7 | \|S\|=16 870, flat(rows)=15 180, flat(diagonals)=14 220, pool 290 (saturated from n=5), window errors 39.9 % (cancelled only at radius N-1), fingerprints 16 648 (nearly injective) |

### VII: detector verdicts

| family | detected verdict | agreement with I-VI |
|---|---|---|
| ring C_n | window (cut capped at 4 from n=6) | yes |
| plane grid | window (row ends [8, 13, 13, 13, 2]) | yes |
| torus grid | window + seam (row ends [8, 61, 112, 50, 2]) | yes |
| referendum | counter (empty cut, threshold-count query) | yes |
| N-queens | register (cut 17 / 59 / 265 / 1146, ×4 growth per step) | yes |

Theorem equality (flat nodes = cut memory motifs): **328/328 levels**.
Counter-hypothesis "projections onto cells still read": only partial
equality (44 to 54 % on grids and N-queens, 83 and 100 % on the trivial
families): it is indeed the cut memory, not the projection.

## Reproducing

Everything is deterministic (Python 3.10+, no dependency):

```
cd benches
python3 study5.py torus 5         # then cylinder 5, plane 5, pressed-torus 5
python3 study6.py referendum 10
python3 study6.py nqueens 7
python3 study6.py spans 6      # then spans 7
python3 study7.py
```

A few seconds each (max RSS 46 MiB). Each bench rebuilds the state space
from scratch, asserts it through invariants (no enumerated conflict, exact
complete solutions, |S| by transfer matrix) then saves into its
results*.json. The values displayed in `studies/` were produced by these
same benches and regenerate identically.

## Conventions

- **flat**: exact positional MDD in the natural cell order;
- **pool**: number of local-signature classes (bounded window), never a
  deployable engine, a structure indicator;
- **cut I(p)**: the set of future cells directly forbidden by the occupied
  cells of the prefix at cut p;
- **query**: aggregative conclusion transition (referendum: majority),
  "integrated" = added to the state space before the flat (flat+query).

## Limits (reminder)

Academic proxies, not the real MCC instances; the 328/328 validation is a
check over complete enumeration, not a formal proof; the profitability of
rungs remains the business of the C4 oracle and of milestone M7
(lisage-bench confrontation, OxiDD/ADIAR witnesses).

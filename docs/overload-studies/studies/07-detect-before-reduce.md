---
name: "study-detect-before-reduce"
title: "Study \u201coverload\u201d VII: detect before reducing"
type: "text/markdown"
---

# Study "overload" VII: detect before reducing

Seventh instalment. Part VI established the fifth law (the span of the
guards in the variable order decides everything) and left a worksite:
turning that law into a controller decision. This part builds and
validates the detector: from the **static reads of the guards** alone,
without enumerating the language, recover the manual verdicts of parts I
to VI and predict the flat exactly.

## 1. Three static readings

A model is seen there as: a variable order, cells, and for each cell q a
guard reading a symmetric set R(q) (the support). Three quantities:

1. **Model radius ρ**: maximum Chebyshev distance between the written
   cell and its reads. Bounded: the guard is evaluable inside a geometric
   window. Growing with the size: window forbidden.
2. **Query shape**: counting fold with threshold over a class of cells
   (referendum: majority): aggregative query, carried neither by the
   window nor by the position.
3. **Cut memory I(p)**: at each cut p of the order, the set of future
   cells directly forbidden by the occupied cells of the prefix, I =
   {v ≥ p : ∃u < p occupied, v ∈ R(u)}. The number of distinct I motifs
   is the quantity the flat must really retain at that cut.

The cut memory is the operational definition of the **register** of part
VI: what the occupation fingerprint (rows, columns, diagonals) encodes
for N-queens, what the thickness-1 band encodes for the grids, nothing
for the referendum.

## 2. The theorem of the bench, and its validation

**Claim**: the number of nodes of the flat MDD at level p equals exactly
the number of distinct I(p) motifs among the real prefixes. Every node of
the flat is a motif of residual forbiddings.

Validation over five families, 328 levels in total, **perfect equality at
100 %**:

| Family | n tested | levels | equality nodes = \|I(p)\| |
|---|---|---|---|
| ring C_n | 3 to 9 | 42 | 42/42 |
| plane grid n×n | 2 to 5 | 54 | 54/54 |
| torus grid n×n | 2 to 5 | 54 | 54/54 |
| N-queens n×n | 4 to 7 | 126 | 126/126 |
| referendum N | 3 to 10 | 52 | 52/52 |

Cross re-checks: the total flats recover exactly the measurements of the
previous parts (plane 5: 249, torus 5: 1 159, N-queens 7: 15 180).

Along the way, the bench refutes a first working hypothesis, formulated at
the launch: the nodes are not the projections of the prefixes onto the
cells still read (equality only 36 to 56 % of the levels). The flat does
not retain the values, only the forbiddings they impose: the projection
survives, the cut memory is the right definition.

Central consequence: **the modern flat is the hash table of the I
motifs**. Analysing the guards is predicting the flat, level by level,
without building the state space. And every annotated reduction (window,
counter, generalized register) is a compression of that table: it can only
claim to group equivalent motifs.

## 3. The families under the detector

**Ring C_n** (part I): the cut memory caps at 4 motifs from n=6 on, a
constant thin band: the plateau of part I, recovered by the sole reading
of the supports. |S| follows the Lucas numbers, the memory does not.

**Grids** (part V): cut memory at the row ends, n=5:

- plane: [8, 13, 13, 13, 2]: constant Fibonacci band from one row to the
  next;
- torus: [8, 61, 112, 50, 2]: same start, then **bulge**.

The seam of part V is statically visible: it is the excess of cut memory
imposed by the wrapping (the guards of the last row re-read the first).
The detector produces the part V law (the seam carries the gain) without
enumerating the language: the possible gain is proportional to the size of
the table the seam inflates.

**Referendum** (part VI-A): cut memory empty at all cuts (free guard), flat
= N exactly, one node per level. The detector sees nothing to reduce in the
dynamics; the majority query alone carries the aggregative flag: counter
verdict, in accordance with part VI (there, the types without
multiplicities were incorrect from N=7 on).

**N-queens** (part VI-B): the cut memory explodes: 17, 59, 265, 1 146
motifs for N=4 to 7 (×3.5 to 4.5 per step), model radius N-1. Register
verdict: the bounded window is declared incorrect before any
construction, in accordance with the error rate measured in part VI (20
to 40 % false positives).

## 4. The verdict table

The classifier reads (radius, query, cut profile) and decides:
aggregative query → counter; bounded radius and thin-band memory →
window; growing radius and growing memory → register.

| Family | ρ | Query | Cut memory | Detected verdict | Historical verdict | Match |
|---|---|---|---|---|---|---|
| ring C_n | 1 | none | capped (4) | window | I: the window/handles carry the gain | yes |
| plane grid | 1 | none | thin band (13) | window | V: window correct, overload not profitable | yes |
| torus grid | 1 | none | band + bulge | window + seam | V: the seam carries the gain | yes |
| referendum | 0 (free guard) | threshold count | empty | counter | VI-A: multiplicities/counter | yes |
| N-queens | n-1 | none | ×4/step growth | register | VI-B: window wrong, register | yes |

Five agreements out of five. The detector does not say whether the
overload is profitable (the plane flat remains unbeatable at small size
while allowing a correct window): it guarantees the correctness of the
available rungs, profitability remaining the business of the C4 oracle
and of milestone M7. An inexact reduction stays disqualified outright:
the detector is the filter that eliminates it before construction.

## 5. Doctrine for lisage-lang

The controller can chain, all statically:

1. collect the read supports of the guards and queries (one pass over the
   guard ASTs);
2. compute the model radius and the aggregate flags;
3. enumerate (or sample) the I motifs along the canonical order: a linear
   pass in the flat construction, or a bounded-budget simulation before
   any decision;
4. choose the rung according to the verdict table, and only then judge the
   profitability between correct options.

Part VI answered "when the guard is global, the window is wrong"; part
VII answers "and the controller can know it before reducing". The triangle
(radius, aggregate, cut memory) is the declaration form of the guards.

## 6. Limits

- Cut memory validated over complete enumeration (small instances); in
  lisage, it is computed by the flat construction itself or by sampling,
  the 328/328 validation does not yet cover that degraded mode;
- variable order fixed (natural, row by row): part VI showed that
  scheduling does not save N-queens (~6 %), but the
  detector-scheduling interaction remains to be explored;
- the detection of the counting fold is declared in the bench, not
  demonstrated over syntactic guards: the AST pattern-matching (Σ with
  threshold over a class) remains to be implemented in lisage-lang;
- academic proxies: real MCC families (the FMS of part III, the
  catalogue Referendum) to be confronted via lisage-bench, milestone M7;
- the cut memory says the size of the flat, not the cost of evaluating
  the transitions (the gain of the EV-MDD counter, cf. part VI).

Data of part VII kept with the project (results7.json: five families,
levels, cut memories and seam profiles; complete bench in under ten
seconds).

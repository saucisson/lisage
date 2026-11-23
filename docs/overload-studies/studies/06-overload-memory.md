---
name: "study-overload-memory"
title: "Study \u201coverload\u201d VI: referendum, N-queens and the memory rung"
type: "text/markdown"
---

# Study "overload" VI: referendum, N-queens and the memory rung

Sixth instalment of the series. Parts I to V had studied languages whose
whole semantics reads inside a bounded window: rings, token protocols,
workflows, 2D grids. There, sharing was a question of cost, and the
"overload" annotation cashed in the regularity (seams, shared levels,
valued edges, clusters). This part attacks the rung left empty in the
taxonomy: **memory**, over two families spotted in the MCC catalogue in
the previous round (Referendum, NQueens), where the pathology is neither
the seam nor the guard span, but the information that is not in the
window.

Two deliberately opposite academic proxies:

- **Referendum**: free dynamics, global query. N voters in a ring (the
  topology of part I), each voter goes from "not voted" to yes or no,
  vote frozen. The dynamics has no guard; the query ("is the ballot
  accepted?") is a counting aggregate over the whole configuration.
- **N-queens**: global filtering guard, trivial query. N×N grid, placing
  a queen requires the absence of a row, column or diagonal conflict with
  any queen present (span N, unbounded); removal free.

Engines as in the previous parts: **flat** = exact positional MDD in
natural order; **pool** = local signature classes (bounded window). Two
instruments proper to memory are added: the classes **with
multiplicities** (multiset of local types, the equivalent of counting
valued edges) and the **guard fingerprint** (occupation register: rows,
columns, occupied diagonals).

## 1. Referendum: the window knows, but only if it is allowed to count

| N | \|S\| = 3^N | flat | flat+query | counter | multiset pool | set pool | amb. set | dec. amb. set |
|---|---|---|---|---|---|---|---|---|
| 2 | 9 | 2 | 7 | 6 | 6 | 6 | 0 | 0 |
| 4 | 81 | 4 | 15 | 15 | 24 | 24 | 0 | 0 |
| 6 | 729 | 6 | 25 | 28 | 127 | 127 | 0 | 0 |
| 7 | 2 187 | 7 | 31 | 36 | 300 | 291 | 6 | 2 |
| 8 | 6 561 | 8 | 37 | 45 | 726 | 702 | 14 | 2 |
| 9 | 19 683 | 9 | 44 | 55 | 1 706 | 1 583 | 74 | 20 |
| 10 | 59 049 | 10 | 51 | 66 | 3 945 | 3 563 | 205 | 34 |

Four readings, two of them counter-intuitive:

1. **The flat is unbeatable**: N nodes (10 for 59 049 states at N=10).
   Language without guard = perfectly shared Cartesian product. Like the
   plane of part V, but in its pure state: the dynamics alone leaves
   nothing to cash in.
2. **The flat absorbs the query without exploding**: adding the "conclude"
   transition (accepted if strict majority of yes, rejected otherwise)
   multiplies the flat by ~N/2 (51 at N=10), not by 3^N. The positional
   MDD pays the counting in a light quadratic, by sharing paths through
   their partial balance. The expected pathology (a "counting blow-up")
   does not exist at the space level.
3. **Types without multiplicities are incorrect**: from N=7 on, the same
   local class (frozenset of neighbour triplets) contains configurations
   with different counts; at N=10, 205 ambiguous classes of which 34 with
   an ambiguous decision, up to 5 count values in one class. The raw
   bounded window does not carry the aggregate: it is a loss of
   expressiveness, not of performance.
4. **Types with multiplicities are correct**: ambiguity 0 for every N,
   because each cell is exactly once the "centre" of a triplet: the
   central margin of the multiset gives the exact count. But the multiset
   nearly re-encodes the whole configuration (3 945 classes at N=10, ×2.4
   growth per step): it is not an engine, it is a proof.

The real rung is the **shared counter**: (number of votes, number of
yeses), i.e. (N+1)(N+2)/2 classes, exactly the valued-edge semantics of
the EV-MDD. It is of the order of magnitude of flat+query (66 against 51
at N=10): here the memory overload does not buy space, it buys a
guarantee: **the counting decision computable locally** (evaluating
"conclude" in O(1) by valued transitions, instead of traversing the
configuration).

Verdict VI-A: for a global aggregative query, the bounded window without
multiplicities is wrong; the multiplicity (counting on the edges) repairs
it. The counter is an expressiveness overload, nearly free in space.

## 2. N-queens: the bounded window is wrong, and nobody compresses

| N | \|S\| | full | flat (rows) | flat (diagonals) | window pool | window errors | fingerprints |
|---|---|---|---|---|---|---|---|
| 4 | 87 | 2 | 135 | 135 | 164 | 20.9 % | 86 |
| 5 | 462 | 10 | 573 | 559 | 290 | 26.6 % | 452 |
| 6 | 2 635 | 4 | 2 896 | 2 786 | 290 | 34.2 % | 2 590 |
| 7 | 16 870 | 40 | 15 180 | 14 220 | 290 | 39.9 % | 16 648 |

Checks: |S| enumerated without any conflict; configurations with N queens
equal the classic solutions (2, 10, 4, 40) at each N, in agreement with
the usual external reference for N-queens counting ([OEIS
A000170](https://oeis.org/A000170)): double validation of the enumeration.

Readings, all inverse of the referendum:

1. **The flat does not explode: it collapses.** At small N it is *bigger
   than the state space* (135 nodes for 87 states at N=4); at N=7 it
   flirts with the list (15 180 for 16 870). The conflict constraint
   crosses the whole chessboard: suffix sharing is nearly zero. Compare
   with the plane of part V: 249 nodes for 55 447 states; the difference
   is not the finesse of the language, it is the **span of the guards** in
   the variable order.
2. **Scheduling cannot help**: sweeping the cells diagonal by diagonal
   gains only 6 % (14 220 against 15 180). Contrary to the FMS of part IV
   (where the Θ(K²) was an order artefact), here no sweep geometry
   localizes a constraint of span N.
3. **The guard memory does not compress the states**: the occupation
   fingerprint (rows, columns, two occupied diagonals) is nearly
   injective (98.7 % at N=7). It is necessary to evaluate the future
   guards, but two distinct configurations almost always keep distinct
   fingerprints, because removal requires the exact position. A separate
   "memory node" does not exist: memory and state coincide.
4. **The bounded window is incorrect, and its plateau proves it**: the
   signature pool saturates at 290 from N=5 on (checked identical at N=6
   and N=7) while the space grows ×6 per step; meanwhile, the
   false-positive rate of the window (it believes an addition legal, the
   semantics refuses it) grows: 20.9 %, 26.6 %, 34.2 %, 39.9 %. A local
   engine would be compact, wrong, and more and more wrong: the worst
   possible assembly.

The span curve closes the diagnosis (share of the (configuration, cell)
pairs mispredicted by a Chebyshev window of radius R):

| radius R | 1 | 2 | 3 | 4 | 5 | 6 |
|---|---|---|---|---|---|---|
| N=6 | 34.2 % | 7.1 % | 1.2 % | 0.07 % | 0 % | |
| N=7 | 39.9 % | 9.3 % | 1.9 % | 0.2 % | 0.01 % | 0 % |

The error decreases by a factor ~4-5 per unit of span, but only vanishes
at R = N-1: the correction span is exactly the size of the grid. This is
structural: a queen threatens the other end of its diagonal.

Verdict VI-B: for a global set filter, the bounded window is wrong (and
its plateau is a sign of blindness, not of mastery), the guard
fingerprint is exact but non-compressing, and the flat alone holds the
language by carrying the memory of the diagonals itself. The useful
overload is not an annex of the diagram, it **is** the diagram.

## 3. The memory rung: two shapes, one diagnosis

The taxonomy of overloads of parts I to V (seam, shared level, valued
edge, cluster synthesis) gains a two-faced rung:

- **Counter** (referendum): the global information is a sum. The overload
  = multiplicities on the edges (valued edges, EV-MDD). Not very costly,
  articulated over the flat, repairs the incorrectness of the window.
- **Occupation register** (N-queens): the global information is a set of
  occupied seats. The overload = the sharing of the levels themselves,
  what the positional construction already does implicitly when the
  variable order cuts the constraints well. It is not separable from the
  state (removals require the position).

In both cases the common lesson goes beyond the two families: **when the
guard (or the query) is global, the bounded window is not expensive, it
is wrong.** Parts I to V never saw this because their guards stayed of
bounded span in the geometry of the variable order; the moment this
condition loosens, the economy of signatures flips from "profitable" (I,
II, torus-V) or "a deficit but correct" (plane-V) to "incorrect" (VI).

## 4. Cross synthesis I to VI

Fifth law of the series, after (I) anchoring alone never suffices, (II)
the flat/pool gap explodes on globally filtered languages, (III-IV)
valued edges and cluster synthesis on dense workflows, (V) the seam, not
the symmetry, carries the gain:

**The span of the guards in the variable order decides everything.**
Bounded-span guards (I to V): the flat shares well or badly depending on
the seams, and the overload annotates, factors, cashes in. Guards or
queries of span N (VI): the sharing of the flat itself depends on the cut
width; the bounded window loses expressiveness; only the memory rung
(counter or register) restores correctness, and it sometimes merges with
the flat.

Consequence for lisage: the controller must classify the guards
**before** choosing the rung: bounded span (window), aggregation (valued
counter), diffuse set (occupation register). A controller that annotated
N-queens the way it annotates Diffusion2D would produce a compact and
wrong diagram.

## 5. Limits

- Academic proxies, not the real MCC instances (the MCC Referendum
  spreads its voters and its decider over a network; NQueens is a
  grid-puzzle): to be confirmed on the real families via lisage-bench;
- the N=8 of N-queens (expected |S| ~100 000) was not enumerated, the
  trends being stable from N=7 on; no closed combinatorial double formula
  for partial |S| (check by invariants: zero enumerated conflict, exact
  complete solutions);
- the window error rate measures the incorrectness of a simple local
  predictor, not of a complete architecture with learned local memory;
- the flat+query of the referendum measures the size, not the cost of
  evaluating the transitions, which is precisely where the valued counter
  wins (Ciardo/Siminiceanu style);
- as always, the pool counts classes, not a deployable engine; milestone
  M7 (OxiDD confrontation, oracle of the C4 profitability criterion)
  remains the arbiter.

## 6. Follow-ups

- Applied, the diagnosis steers the follow-up: the automatic
  classification of the guard span ("window / counter / register"
  detection at the lisage-lang controller level) becomes the natural
  candidate for part VII, with the real FMS of part III as testing ground
  for the memory rung;
- lisage-bench M7 confrontation: measure whether the valued edges
  (counter) reduce the evaluation of the transitions of the real
  referendum of N voters, as verdict VI-A suggests.

Data of part VI kept with the project (results6.json: referendum N=2 to
10, N-queens N=4 to 7, window spans N=6 and N=7; bench a few seconds, max
RSS 43 MiB).

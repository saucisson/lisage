---
name: "study-2d-grids"
title: "Study \u201coverload\u201d V: 2D grids, crossing seams and the corner question"
type: "text/markdown"
---

# Study "overload" V: 2D grids, crossing seams and the corner question

Fifth instalment of the series of studies on the "overload" annotation in
lisage decision diagrams. After the rings (I), the protocols (II), the
dense workflows (III) and the FMS++ synthesis (IV), this part goes 2
dimensions: it is the exact counterpart of the philosophers of part I,
where the translative symmetry of the chain becomes a grid symmetry, and
where the wrapping seams cross.

## 1. Model and boundary variants

Language under study: independent sets of an n×n grid. Cell {0,1}, local
guard (adding a 1 requires the 4 neighbours at 0), removal free. From the
empty state, all independent sets are reachable.

To isolate the proper effect of each wrapping seam, four boundary
variants:

| Variant | Wrapped axes | Seams | Orbit symmetry |
|---|---|---|---|
| torus | both | 2 crossing seams + corner | n² translations |
| cylinder | rows axis | 1 seam | n translations |
| plane | none | free borders | none |
| pressed torus | both | same as torus + singularity | anchor = irregular cell |

The "pressed" torus takes over the protocol of part I: cell (0,0) has
alphabet {0,1,2} (singleton module, the 2D "left-hander"). The singularity
breaks the translation: the anchor is the irregular cell itself, no orbit
is legitimate any more.

Engines as in the previous parts: **flat** = exact positional MDD in
row-by-row order; **pool** = number of distinct signatures (capped
distances to the 4 borders, W=2; anchor letter; 5-cell window; -1
sentinels outside the wrapping) over configurations anchored by canonical
translation (lex-min). The pool counts the classes of the annotated
engine, not a deployable engine.

**Corner question** (the motivation of part V): when two wrapping seams
cross, does the corner cell become a singularity that forces paying one
anchor per corner, or is the corner benign? Protocol: signatures
partitioned into **A** (fully capped cell, interior) and **B** (touching at
least one capped border).

## 2. Torus: two crossing seams

| n | \|S\| | anchored | flat | pool | A (int) | B (edge) |
|---|---|---|---|---|---|---|
| 2 | 7 | 3 | 8 | 11 | 0 | 11 |
| 3 | 34 | 6 | 39 | 43 | 0 | 43 |
| 4 | 743 | 56 | 225 | 158 | 0 | 158 |
| 5 | 25 531 | 1 027 | 1 159 | 300 | 17 | 283 |

Successive flat ratios: 4.9 / 5.8 / 5.1. Constant ×5.5 rate: this is the
"border swell" law of part I, but with a base ~5.5 instead of ~2: in
wrapped 2D, the flat swells exponentially in the width. The pool, for its
part, clearly decelerates (×3.9 / ×3.7 / ×1.9): it converges towards its
saturation plateau, as for the philosophers.

At n=6, the transfer matrix gives the exact |S| without enumeration:
2 406 862 (i.e. ×94 compared to n=5, while flat only grows ×5.5 and the
pool ×2). Complete enumeration at n=6 exceeded the bench budget; the exact
count is nevertheless validated by double computation (BFS and transfer
matrix coincide at each checked n).

## 3. Cylinder then plane: a single seam suffices, zero seam inverts the trend

**Cylinder** (1 seam, rows axis):

| n | \|S\| | anchored | flat | pool | A | B |
|---|---|---|---|---|---|---|
| 2 | 7 | 4 | 8 | 13 | 0 | 13 |
| 3 | 43 | 15 | 43 | 52 | 0 | 52 |
| 4 | 933 | 244 | 241 | 174 | 0 | 174 |
| 5 | 36 211 | 7 243 | 1 407 | 299 | 17 | 282 |

Flat ratios: 5.4 / 5.6 / 5.8. Nearly identical to the torus, everywhere:
same exponential rate, same pool (299 against 300), same interior
classes. **Adding the second (crossing) seam costs nothing**: the corner of
the torus is benign.

Exact |S| at n=6 (transfer): 3 557 711.

**Plane** (free borders, no seam):

| n | \|S\| | anchored | flat | pool | A | B |
|---|---|---|---|---|---|---|
| 2 | 7 | 7 | 8 | 22 | 0 | 22 |
| 3 | 63 | 63 | 32 | 107 | 0 | 107 |
| 4 | 1 234 | 1 234 | 95 | 277 | 0 | 277 |
| 5 | 55 447 | 55 447 | 249 | 519 | 34 | 485 |

Flat ratios: 4.0 / 3.0 / 2.6, clearly decreasing: without a seam, the flat
becomes nearly polynomial (249 nodes for 55 447 states, record of the
series). And the pool grows faster than the flat: **from n=3 on, 107
classes for 32 flat nodes**; at n=5, 519 against 249. Complete inversion:
without a seam, there is nothing to cash in, the annotation is a loss.
Translation is moreover no longer a symmetry (no orbit), so anchoring
itself no longer makes sense: the plane is simply the flat's territory.

## 4. Pressed torus: the singularity that pays for the pool, free for the flat

| n | \|S\| | flat | pool | A | B |
|---|---|---|---|---|---|
| 2 | 9 | 8 | 29 | 0 | 29 |
| 3 | 41 | 39 | 143 | 0 | 143 |
| 4 | 920 | 225 | 535 | 0 | 535 |
| 5 | 31 213 | 1 159 | 978 | 51 | 927 |

Two counter-intuitive results:

1. **Flat strictly identical to the torus** (8/39/225/1 159) while |S|
   grew by 22 % (25 531 → 31 213). The letter 2 forces its neighbours at
   0, so the sub-tree downstream of value 2 is a constrained copy of a
   sub-tree that already existed: the positional MDD shares it for free.
   The flat absorbs the overload without knowing it.
2. The pool, for its part, pays ×3.3 (978 against 300). In detail:
   interior A = 51 = 3 × 17 exactly, the anchor letter multiplies every
   class; edge B = 927 against 283. The chosen signature (5-cell window
   around the current cell) does not see the distance to the singularity:
   it is under-dimensioned to absorb it.

This is the **exact inversion of the pressed philosophers of part I**
(where the annotation reduced the entries from 18 to 13). There, the
pressing extended the edges and the anchor-singularity cashed in the
regularity; here, the letter 2 constrains and the flat benefits, while
the pool, deprived of a legitimate translation, anchors every
configuration on the singularity and pays full price.

Design lesson: profitability of the overload ≠ property of the overload
alone, but of the trio (overload, guard, engine). A letter that
constrains enriches the flat; a letter that extends enriches the classes.
And an anchor is useful only if it factors: anchor-singularity without an
orbit = a configuration counter, not a compressor.

## 5. Answer to the corner question

**The corner is benign.** Three converging proofs:

- **A** (interior signatures, fully capped) exists only from n=5 on (first
  cell at distance ≥ 2 from the four borders) and stays tiny: 17
  (torus/cylinder), 34 (plane), 51 (pressed). Over the 4 variants, A caps
  from its appearance.
- **B** (signatures touching a border) concentrates the variety (more
  than 92 % of the pool at n=5) but its growth decelerates everywhere
  (last ratio ~1.75-1.9): polynomial residual growth along the perimeter,
  not a singularity. The pool caps because the signature space is finite
  by construction; the convergence is already visible.
- **Cylinder ≈ torus** on all the metrics: crossing a second seam over
  the first adds neither rate, nor classes, nor any nuisance metric. The
  dreaded singularity of the corner does not exist.

Consequence for the canonical construction: no need for multiple anchors
per level or corner detection at the controller. A single seam suffices to
trigger the swell, and a single anchor suffices to cash it in. The
controller only needs to detect the presence/absence of wrapping, not its
fine geometry.

## 6. Cross synthesis I to V

Fourth law of the series (after: anchoring alone never suffices; the
flat/pool gap explodes on globally filtered languages; valued edges are
the useful overload of dense workflows):

**In 2D, the seam, not the symmetry, carries the gain.** Translative
symmetry (large orbits) is already absorbed by classical anchoring; what
makes the flat swell exponentially (×5.5/step) is the wrapping seam, and
one suffices. Symmetrically, where there is no seam (plane), the flat is
nearly polynomial and the annotation becomes a deficit from n=3 on.

| Variant n=5 | flat | pool | flat/pool |
|---|---|---|---|
| torus | 1 159 | 300 | 3.9 |
| cylinder | 1 407 | 299 | 4.7 |
| pressed torus | 1 159 | 978 | 1.2 |
| plane | 249 | 519 | 0.5 |

Ranking of overload taxonomies completed: topological seam (I, II, V),
shared level (II), valued edge (III, IV), cluster synthesis (IV), and now
two precious counter-examples: benign overload for the flat (constraining
letter, V) and non-factoring anchor (singularity without orbit, V). The
lisage controller must classify the profitability of the annotation on the
trio (overload, guard, engine) and not on the regularity of the model
alone.

## 7. Limits and follow-ups

Usual limits, plus specific ones:

- n ≤ 5 in complete enumeration (n=6 torus: 2.4 M states, beyond the
  Python bench budget); the exact n=6 counts by transfer matrix are still
  validated by double source at the n where enumeration goes through;
- the pool is a class counter, not a deployable engine: the question of
  detection at the controller (the "geometry" part of the series) remains
  open on the real pipeline;
- the pressed signature (5-cell window) is under-dimensioned for the
  singularity: enriching the window with a distance to the anchor would
  reduce B, to be tested;
- academic proxy (independent sets), not a real MCC model: the 2D
  diffusions of the MCC (Diffusion2D) add continuous quantities and
  rectangular grids, which leaves the bench;
- to be confirmed in lisage-bench at milestone M7 against OxiDD, as for
  the previous parts.

Announced follow-up (part VI): families with memory overload, Referendum
and NQueens, where the pathology is neither the seam nor the guard span
but the history (the missing rung).

Data and script of part V kept with the project (results5.json, four
variants n=2 to 5, max RSS 25 MiB, under 2 s per n).

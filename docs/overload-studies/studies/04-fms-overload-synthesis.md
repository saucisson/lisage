---
name: "study-fms-overload-synthesis"
title: "Study \u201coverload\u201d IV: FMS — cluster synthesis and valued edges"
type: "text/markdown"
---

# Study on "overload" — part IV: FMS, can we do better?

Return on the anomaly of part III: the FMS was the only workflow whose
flat nodes grew in ~12K² (the others are linear). Question: is that a
hardness of the model, or an artefact? And can the overload annotation
idea, in another form, do better than the seam and the valuation?

## 1. Diagnosis: a scheduling artefact, not a hardness

The FMS-lite stage constraint (a + b + machine-busy ≤ K) bears on three
variables that the flat order interleaves: (a0, a1, a2, b0, b1, b2, m0,
m1, m2). The MDD must therefore carry the context (partial sums (a_i, b_i))
across the intermediate levels: Θ(K²) node classes. The proof that this is
not intrinsic: the **reachable space factors exactly**, even after adding
a **shared crane** (capacity C between the stages):

**|S| = f(K)³ · g(C)², checked exactly at the three points**: f(K) =
T(K+1)+K(K+1) stage states (5, 12, 22), g(C) = (C+1)(C+2)/2 lift states.
Measured: 4 500, 62 208, 383 328 with C = 2. The crane couples the
transitions, not the states.

## 2. The ladder of representations (K = 1, 2, 3 ; C = 2)

| rung | representation | nodes (K=1/2/3) | edges (K=1/2/3) | law |
|---|---|---|---|---|
| R0 | interleaved flat (the order of part III) | 53 / 92 / 155 | 96 / 199 / 410 | Θ(K²) |
| R1 | contiguous clusters (stage, lift, stage, lift, stage) | 23 / 26 / 29 | 45 / 57 / 72 | Θ(K) |
| R2 | **aggregated per cluster** (5 super-variables) | **5 / 5 / 5** | 27 / 48 / 78 | **O(1) nodes** |
| R3 | R2 + **valued edges** | 5 | **5** | O(1), everything |

The three measured slopes: R0 = quadratic (51 nodes of apparent slope over
3 points), R1 = 3K+20, R2 = 5, exactly 5 at every K — one node per cluster
plus the terminal.

## 3. The two new overloads

What part IV adds to the overload annotation idea:

- **Synthesis overload (R2).** Here the annotation does not break the
  homogeneity of a level: it **contracts a cluster of variables into a
  single node annotated with its composite alphabet** — the stage (a, b, m)
  becomes ONE variable with f(K) values, the crane lift (x, y) a variable
  with g(C) values. In lisage terms: this is the synthesized transduction —
  the module closes its contract locally and exports only its interface.
  Nodes: Θ(K²) → 5.
- **Valued overload (R3).** The edges remain: the stage node has f(K) ≈
  Θ(K²) sibling edges (one per value of the composite alphabet). A range
  transduction folds them into a single annotated edge. Edges: Θ(K²) → 1
  per node, 5 in total. This is the lineage of valued edges, already named
  in part III.

Combined: **5 nodes and 5 annotated edges to represent 383 328 states**
(at K=3, C=2) — and the size of the representation no longer depends on K
at all.

## 4. Detection at the controller: the criterion is the span

The static signal announcing the whole affair: the **span of the guards** —
the maximum distance, in the variable order, between members of the same
constraint. In R0, the stage constraint has a span of 6 (a0 … m0) and the
machine→crane unloadings cross nearly the whole file. Classifier rule that
follows, readable before construction:

1. small span and clusters already contiguous → nothing to do (the other
   workflows of part III);
2. wide spans over identifiable clusters → **reschedule**, free (R1:
   factor 5 to 6 on the nodes of the small instances, and the change of law
   K² → K);
3. cluster endowed with a composite alphabet locally closed → **synthesis
   overload** (R2: law K → 5);
4. remaining range edges → **valued overload** (R3: 5).

Same moral as the previous parts: the plan comes out of the controller
before any BFS, here by simple reading of the guard spans.

## 5. Answer to the question, and its limit

**Yes, we can do much better on the FMS** — but the better is not a seam:
it is the **contraction**. The taxonomy of overloads, completed:

| overload | target | part |
|---|---|---|
| topological seam | module/marking asymmetry over a regular topology | I, II |
| shared level (victims/token) | circulating global object | II |
| valued (edge range) | marking width | III, IV |
| **synthesis (cluster → 1 annotated variable)** | locally closed constraints shattered by the order | **IV** |

The honest limit: this FMS-lite factors (the exact assert proves it), so
R2 falls back onto a pure product. The **real** Ciardo FMS keeps pools of
machines shared by several flows (machines M1 to M4 serve operations of
different stages): factorization there will never be total. The missing
rung, planned but not measured here, is the **memory overload**: carrying
the residual capacity of the pool as an edge attribute (lisage memory
transduction) between aggregated clusters — a few nodes, a few edges, one
counted attribute. This is the manipulation to bench first in
`lisage-bench` (M7) on the real PNML of the MCC FMS, with the R0→R3 ladder
as measurement protocol.

## 6. Limits

- Toy: 3 stages, two-counter crane, K ≤ 3 (the explicit BFS tops out at
  ~4·10⁵ states here); the constants (5, 27, 78) are those of the proxy.
- The R2 edges (27, 48, 78 = f(K)+g(C)+f(K)+g(C)+f(K)) follow the alphabet
  sizes exactly: closed-form measurement confirmed.
- The span is a heuristic criterion: it detects the scheduling and
  aggregation opportunity; the choice between R1 and R2 plays out on the
  size of the composite alphabet vs the carried context, to be settled by
  the profitability oracle (C4), not by a fixed rule.

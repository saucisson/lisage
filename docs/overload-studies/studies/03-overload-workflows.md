---
name: "study-overload-workflows"
title: "Study \u201coverload\u201d III: dense token workflows (Kanban, FMS & co)"
type: "text/markdown"
---

# Study on "homogeneity overload" — part III: Kanban, FMS, HouseConstruction, SwimmingPool, ViralEpidemic

Third scaling morphology: here, the parameter grows the **marking width**
(the counter values), not the topology. Kanban and company are production
queues with bounded capacity, not module networks. The measurement protocol
adapts: we measure **edges** in addition to nodes, because that is where
the cost lives.

## 1. Families (proxies faithful to the spirit of the MCC models)

| Family | Proxy | Parameter | Variables |
|---|---|---|---|
| kanban | 3 stages in a line, (waiting, done) per stage, kanban capacity K | K | 6 counters |
| fms | 3 stages, 2 part types A/B share one machine per stage | K | 9 variables |
| house | chain of 4 construction stages, H house-tokens, conservation | H | 5 counters |
| pool | (queue, basin ≤ 3, exited), H swimmers | H | 3 counters |
| viral | (S, I, R), S+I+R = N, init (N−1, 1, 0) | N | 3 counters |

All invariants checked at each point: kanban capacity, FMS stage capacity,
conservation (house, pool, viral), basin ≤ 3.

## 2. Results

**Kanban** — |S| = T(K+2,2)³ (27, 216, 1000, 3375, 9261: cube of the
triangular), nodes = 3K+6:

| K | \|S\| | nodes | edges |
|---|---|---|---|
| 1 | 27 | 9 | 15 |
| 3 | 1 000 | 15 | 42 |
| 5 | 9 261 | 21 | 81 |

**FMS** — |S| = (T(K+1)+K(K+1))³ (125, 1728, 10648), nodes ≈ 12K²+3K+30:

| K | \|S\| | nodes | edges |
|---|---|---|---|
| 1 | 125 | 45 | 78 |
| 2 | 1 728 | 84 | 181 |
| 3 | 10 648 | 147 | 392 |

**HouseConstruction** — |S| = C(H+4,4) (5 … 495), nodes = 4H+5:

| H | \|S\| | nodes | edges |
|---|---|---|---|
| 4 | 70 | 21 | 55 |
| 8 | 495 | 37 | 153 |

**SwimmingPool** — |S| = 2H²+2, nodes = 2H+3:

| H | \|S\| | nodes | edges |
|---|---|---|---|
| 4 | 34 | 11 | 34 |
| 8 | 130 | 19 | 84 |

**ViralEpidemic** — |S| = N(N+3)/2, nodes = 2N+2:

| N | \|S\| | nodes | edges |
|---|---|---|---|
| 10 | 65 | 22 | 86 |
| 40 | 860 | 82 | 941 |

## 3. Reading: the stake is neither the nodes nor the fold

1. **The flat holds very well.** Nodes linear or nearly so in the parameter
   (3K+6, 2H+3, 2N+2…), a few dozen nodes for |S| exploding into a
   polynomial of degree 2 to 6: the |S|/nodes ratio already reaches 441 on
   the toy kanban. Contrary to petN (part II), nothing goes super-linear:
   the staged structure keeps simple coupled constraints, portable into the
   flat table.
2. **The fold has nothing to fold.** No class of repeatable modules: every
   variable has a distinct role. The classifier renders "empty plan", and
   it is right: the windowed quotient would be identical to the flat.
   Zero-injection applies, by uselessness and not by necessity.
3. **The real cost is the edges.** Edges grow in Θ(param²) (15 → 81 for
   kanban K=5; 941 for viral N=40): it is the marking width that pays, edge
   by edge, value by value (one edge per counter value). At MCC scale
   (bounds k in tens, hundreds), Θ(K²) edges becomes the dominant cost
   line.

## 4. The overload that applies here: valued, not topological

For these families, the useful annotation is not a module seam but a
**valued edge annotation**: the K+1 sibling edges of a node (one per
counter value) fold into a single annotated edge carrying a range
transduction. Measurement: annotated edges = the number of nodes, hence
exactly the "annotated edges" line of the tables (9 vs 15 at K=1; 82 vs 941
at N=40: **×11 fewer edges**, and the ratio grows linearly with the
parameter: ~param/2).

This is the lineage of the valued edges of Ciardo's EV-MDD ([Ciardo &
Siminiceanu](https://link.springer.com/chapter/10.1007/3-540-36126-X_16)) —
and it is natively lisage ground: the attributed transductions of edges are
already the engine format. Part III therefore says: on these families, the
overload to plan in lisage is not topological but **dense**, and it is
already in the language.

## 5. Synthesis of the three parts

| | I. Rings (Philo, Token) | II. Protocols (petN) | III. Dense workflows (Kanban & co) |
|---|---|---|---|
| What grows | the topology | the universal guards + n−1 levels | the marking width |
| flat | Θ(n) | super-linear (explodes) | Θ(param): holds! |
| window / fold | **O(1)**: plateau | polynomial: crushes the flat | useless (nothing to fold) |
| crossing | n ≈ 30 | n ≈ 3 | none: the flat suffices |
| useful overload | topological seam | victim / token level | **valued edges** (×param/2 fewer edges) |
| classifier verdict | semantic anchor plan | 1 class modulo rotation | "nothing to do" |

Consequence for the MCC ranking: Kanban, FMS, HouseConstruction,
SwimmingPool and ViralEpidemic form a **third category**, distinct from the
rings (winning O(1)) and the protocols (crushing polynomial): models where
every DD engine scores easy points at the StateSpace exam (|S| in the
billions, of which the explicit engine sees nothing), and whose
differentiator is not the number of nodes but the way of paying the edges:
the lisage edge there is its transductions, not its seams.

## 6. Limits

- Reduced proxies: the real MCC Kanban and FMS have more places (Ciardo's
  FMS has ~22); the exact degrees change, not the laws (nodes ≤
  Θ(param), edges Θ(param²), |S| high-degree polynomial).
- Real range of the MCC instances: K up to several tens and slightly
  coupled structures between stages; the closed forms above remain to be
  confirmed on the real PNMLs.
- The quadratic FMS node (12K²+3K+30) shows that per-machine coupling
  degrades the flat: the line between "easy workflows" and "hard
  protocols" is blurry and plays out case by case on the shared guards.

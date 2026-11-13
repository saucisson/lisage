# Golden-thread example: n philosophers, end to end

*Complete walkthrough of the framework on the regular case: model, controller, encoding, transductions, saturation, operations, then the irregular variant of the left-handed philosopher (§ 9). Figures verified by simulation: exhaustive membership for n ≤ 7, counting against the indein-flightence polynomial of the cycle up to n = 20, measurements of the flat prototype up to n = 200 (§ 6.2, § 9.5). Reported results: the reachable space, exponential in n, fits in 7 nodes indein-flightent of n; the intermediate blow-up is in the naive policy, not in the structure; the variant with one irregular philosopher also fits in 7 nodes.*

---

## 0. The result in one table

| Representation of R_n | Nodes | n = 50 |
|---|---|---|
| Expallowed enumeration | ≈ (1+√3)^n ≈ 2.73^n states | ≈ 7·10²¹ states: impossible |
| Flat MDD, fixed order, § 2.3 reductions only | Θ(n) | ~200 |
| Hierarchical SDD (Hamez) | compact: the hierarchy captures the regularity (it is its textbook case) | to be measured (milestone 4) |
| **Flat Lisage + relative forms + D-abs** | **7 nodes, for all n ≥ 3** | **7** |

The 7 nodes: 5 forms + 2 terminals. The parameter n lives in no node: it lives on the shift of the input edge (the unfolding). R_5 and R_50 share exactly the same physical nodes; changing n means changing the anchor, not the graph. The 5 forms are the minimal automaton of the language of legal cells: C1 (minimality by handle) requires these nodes, not one fewer, not one more. Checks: |R_3| = 20, |R_4| = 56, |R_5| = 152, |R_6| = 416, |R_8| = 3104, |R_20| = 536 754 176. The irregular variant (§ 9) also fits in 7 nodes: only the root node changes (7 keys instead of 5), the interior is unchanged.

---

## 1. The level-1 model (the language)

```
type Philo = Tag { think, hungry, eat }
type Fork  = Tag { free, me, prev }        // me : held by the philosopher of the cell ; prev : by the previous one
type Cell  = { pc : Philo, left : Fork }    // left : the fork between i-1 and i
type State = Seq Cell                       // ring of length n, n ≥ 3

take    : ∀i, pc_i = hungry ∧ left_i = free ∧ left_{i+1} = free
          → pc_i = eat, left_i = me, left_{i+1} = prev     // atomic seizure of both forks
release : ∀i, pc_i = eat → pc_i = think, left_i = free, left_{i+1} = free
hungry  : ∀i, pc_i = think → pc_i = hungry

let init     = map (i -> { pc : think, left : free }) (ring n)
let reach    = fix (take ∪ release ∪ hungry) init    // saturation, auto-detected policy
let n_states = count reach                            // semiring (ℕ,+,×) : 152 if n = 5
let stuck    = filter (s -> ¬ enabled s) reach        // empty : think → hungry is always armed
```

Atomic seizure: no deadlock in this model. The "one left-handed philosopher" variant (two-step seizure) is the falsifiable testbed: delivered in § 9.

---

## 2. The C_philo controller (instantiation of § 1.2 of the proposition)

```
C_philo = ( δ           : bounded, Tag{think,hungry,eat} × Tag{free,me,prev}   // 9 keys, 5 allowed placements
          , Σ            : Seq(Cell), length n ≥ 3
          , decomposition + O   : positional : cell i before cell i+1 ; the ring is unrolled,
                          the 0/n seam is expallowed (root = position 0, anchor = position n)
          , R            : R-collapse, R-default, R-share (spec § 2.3) ; D-abs, D-trans armed (spec § 2.4)
          , defaults     : deduced from the type : the cell (think, free)
          , D^def        : (think, free), absorbable on any subdomain
          , encoding     : relative in the interior (shift on the edge) ; absolute at the seams
          , attribution   : inherited = (p, shift), p = "the previous one eats" ;
                            synthesized = semiring (Bool for saturation, (ℕ,+,×) for counting) ;
                            memory = cache (event, form, anchor)
          )
```

The choice that carries everything else: the **interleaving** (pc_i, left_i). Each fork lives in the cell of its right-hand consumer, so the three events are local (width ≤ 2, § 5). Encoding variant "Seq Philo then Seq Fork": take_i becomes non-local and requires level memory (T3); the framework can express it too, but the profitability oracle prefers the interleaving. **The locality of events is a property of the instance, not of the framework.**

---

## 3. The reachable space as a language

Fork/philosopher consistency is local to the cell (inclusion typing: Val(x) dein-flights on the inherited p):

| Inherited p ("the previous one eats") | Legal cells (pc, left) |
|---|---|
| p = 0 | (think, free), (hungry, free), (eat, me) |
| p = 1 | (think, prev), (hungry, prev) ; (eat, ·) not placed |

A state = a sequence of n legal cells with no two circularly adjacent "eat"s, and:

> **R_n = circular words of n cells with no adjacent "eat eat" = 2^n · I(C_n, ½)**, with I the indein-flightence polynomial of the cycle.

Proof in two steps (verified by expallowed BFS up to n = 8): every reachable state respects the invariant (the guards impose it); every indein-flightent configuration is reached (seizures in any order, then hungry transitions). The bijection pc-words ↔ (E, H): E = eaters (indein-flightent in the cycle), H = hungry ones, the rest think.

---

## 4. The representation: 5 forms, 2 terminals, 1 anchor

The reference SVG diagram is in the canvas "The 7 forms of the philosophers: SVG diagram"; the ASCII below remains the source of truth for the keys.

```
root S (position 0, absolute encoding ; its left key already encodes pc_{n-1})
  ├─ (t,f) ──▶ A₀        ┐
  ├─ (h,f) ──▶ A₀        │ promise : pc_{n-1} ≠ eat
  ├─ (e,m) ──▶ A₁        ┘ (input p = 1 : I eat)
  ├─ (t,p) ──▶ B₀        ┐
  └─ (h,p) ──▶ B₀        ┘ promise : pc_{n-1} = eat

interiors (cells 1..n-1, shift on the edge, anchor = position n)
  A₀ : (t,f)→A₀  (h,f)→A₀  (e,m)→A₁        B₀ : (t,f)→B₀  (h,f)→B₀  (e,m)→B₁
  A₁ : (t,p)→A₀  (h,p)→A₀  (e,·) not placed   B₁ : (t,p)→B₀  (h,p)→B₀  (e,·) not placed
  at the anchor : A₀ → ⊤, A₁ → ⊥            at the anchor : B₀ → ⊥, B₁ → ⊤
```

Reading the mechanisms:

- **Handle**: (C_philo, form, anchor). The anchor is symbolic ("the end of the ring"); the shift lives on the edge, +1 per cell; R_n = unfolding of S over n cells. The same nodes serve all values of n.
- **Relative forms (spec § 2.4.3)**: A₀ and B₀ are the self-similar pattern "one cell, then the rest"; their (t,f) loop is the regular segment, unfolded up to the anchor. Unfolding A₀ gives exactly what the SDD hierarchy encodes in one node: the unfold conjecture (spec § 2.4.4) in action.
- **D-abs (spec § 2.4.1)**: (think, free) = D^def; a run of default cells of any length reads as the single edge (t,f). The initial state (everyone thinks) = one edge, not a tree.
- **Inclusion typing**: the (eat, ·) edge of A₁/B₁ does not exist: the key is not in Val(x) when the previous one eats.
- **Seam**: the fork of cell 0 already encodes pc_{n-1}, so S reads a **promise** about the end of the ring, and the anchor verifies it. Line A (promise "the last one does not eat") is the core; line B (promise "the last one eats") and the terminal ⊥ are the boundary term: O(1). A₀ and B₀ have the same child pattern, they differ by the anchor verdict: distinct under C1, merged by no quotient.
- **Minimality**: S, A₀, A₁, B₀, B₁ = the 5 states of the minimal automaton of the language (start + (p, promise)); canonicity is coalgebra minimization (§ 1.3 of the proposition), this document is its concrete verification. Verified on November 10, 2026: the 5 states are pairwise distinguished by suffixes of length ≤ 2 (the only witness of length 2 is S ≠ A₀, suffix [(t,p),(e,m)]: only S reaches line B from the input); no mergeable node, not one more, not one fewer.

Three control readings (n = 5): eat at 2 only: S→A₀, (e,m)→A₁, (t,p)→A₀, loops, anchor(A₀) → ⊤. Eat at 0 only: S→A₁, (t,p)→A₀, loops, anchor(A₀) → ⊤. Eat at 0 and 4: S→A₁, (t,p)→A₀, loops, (e,m)→A₁, anchor(A₁) → ⊥ (0 and 4 are adjacent in the ring: rejected). These three traces are in the exhaustive simulation.

---

## 5. Events as attributed transductions

| Transduction | Width | Channels | Scope signature |
|---|---|---|---|
| hungry | 1 cell | inherited (p, shift) | reads (i) |
| take | 2 cells | inherited (p, shift) ; upward guard | reads (i, i+1) |
| release | 2 cells | inherited (p, shift) | reads (i, i+1) |

The 3n events = **3 classes × n shifts**: each class is invariant under translation, the transduction descends with the shift in the inherited channel, the computed cache carries the key (event, form, anchor). The work is done once per pair, not once per position: this is work sharing by translation invariance (spec § 2.4). The seam events (take/release at positions 0 and n-1) are the only ones writing to line B: the boundary, still O(1).

---

## 6. Saturation: O(1) core + seam

Start: init = one edge (D-abs). Fixpoint: the 7 nodes, in three to four rounds:

1. hungry everywhere: (h,f) edges on S, A₀, B₀;
2. take: (e,m) edges on A₀, B₀ and S; discovery of A₁, B₁;
3. take/release near the seam: keys (t,p), (h,p) of S (discovery of line B) then of lines A/B; convergence.

The "static by levels" policy (Ciardo) sweeps positions; here the positions are gone: the iteration runs over uses (event × form × anchor), the level index → uses living in the shadow plane (the carrier lane discussed for Hamez compatibility). The "auto-detected" policy (Hamez) recognizes the regular pattern and does exactly this sweep by forms. Expected cost: constant per event class + the seam. This is the falsifiable prediction of contribution 8: if saturation costs Θ(n) per class, work sharing is wrong.

### 6.1 What a firing does (the elementary operation)

A firing of event e on the current node (image, then union):

1. **descent**: the transduction follows the inherited channel down to position i; in the flat prototype these are i levels, in Lisage the shift is already on the edge and the form is re-read in the pool;
2. **guard**: reading at most 2 keys (take: (h,f) ∧ (·,f)); a missing key cuts the branch;
3. **local rewrite**: re-keying at most 2 nodes ((h,f)→(e,m) and (·,f)→(·,p));
4. **union** with the partial result, then re-reduction: hash-consing merges equal nodes at write time (free R-share);
5. **cache**: key (event, node) in the prototype, (event, form, anchor) in Lisage: the second firing on the same pair is free.

The seam: in the flat version, take_{n-1} re-reads position 0 from depth n-1, a descent of span n paid at each application; in Lisage the seam is an edge of the pool, O(1). This is the only per-firing complexity gap between flat and Lisage.

### 6.2 Measured: where it blows up (and where it does not)

Prototype: hash-consed flat MDD in Python, fixed order, same events; oracles: expallowed BFS (sat == BFS for n = 4..7) and fold (== exact counting up to n = 20). Measurements from an interpreted prototype: the ratios and orders of magnitude are meaningful, the constants are not.

| n | 5 | 10 | 20 | 30 |
|---|---|---|---|---|
| final result (live nodes, ⊤ included) | 16 | 36 | 76 | 116 |
| saturation: max image in progress | 15 | 35 | 75 | 115 |
| naive BFS: max image in progress | 26 | 126 | 553 | 1 276 |
| saturation: nodes created (unique table) | 140 | 655 | 2 885 | 6 715 |
| naive BFS: nodes created | 432 | 4 634 | 44 959 | 162 884 |
| saturation: firings / passes | 45 / 2 | 90 / 2 | 180 / 2 | 270 / 2 |
| naive BFS: firings / rounds | 120 / 8 | 480 / 16 | 1 860 / 31 | 4 140 / 46 |

Three readings:

- **The intermediate blow-up exists, and it is in the policy, not in the structure.** The final result is linear (4n − 4), but naive BFS swells its in-progress image to ≈ 1.4 n²: up to 11 × the final result (1 276 vs 116 at n = 30). Saturation keeps the image ≤ result + 1: it never writes much more than it keeps.
- **The creation cost follows**: ≈ 7.8 n² in saturation (Θ(n²): each (event, node) pair written at most once), ≈ 6 n³ in naive. In the Lisage pool, these pairs are quotiented into (event, form, anchor): the target is O(1) per class + seam (contribution 8).
- **The quotient is already visible in the flat version**: the 15 live internal nodes at n = 5 carry only 5 distinct signatures, with multipallowedies ×6, ×5, ×2, ×1, ×1: the flat version stores positional replicas of the 5 forms of § 4 (⊥ is impallowed there: key absent). The table of § 0 is thus half measured: flat pays n copies, Lisage pays one form and bills the unfolding at read time.

(At n = 100: 396 live, 78 325 created, 900 firings, 2 passes; at n = 200: 796 live, 316 625 created.)

### 6.3 The same exercise "DDD-style" and "Hamez-style": what the policy changes, what it does not

The prototype **is** a DDD-style homomorphism engine (Couvreur): local application of an event, memo (event, node), memoized union (node, node), idempotent hash-consed merge ⊕. The figures of § 6.2 therefore read as-is as DDD measurements. Complementary measurements (rebuilt engine, consistent to ±2 % with § 6.2): the **live structure peak** (what an engine with GC and evictable caches must actually hold in flight: root + current image) and the **memory retained without eviction** (unique table + the two memos, never purged):

| n | 10 | 20 | 30 |
|---|---|---|---|
| **Saturation**: live peak | 68 (≈2.0 × final) | 148 (≈1.95) | 228 (≈1.97) |
| **Saturation**: retained without eviction | 2 033 (≈20 n²) | 9 003 (≈23 n²) | 20 973 (≈23 n²) |
| **Naive fixpoint**: live peak | 299 (8.3 × final) | 1 299 (17.1 ×) | 2 971 (25.6 ×) |
| **Naive fixpoint**: retained without eviction | 14 942 (≈15 n³) | 142 692 (≈18 n³) | 513 092 (≈19 n³) |

Three readings:

- **The live peak is governed by the policy, not by the representation.** In saturation, the engine never holds much more than the result (≈2×: the result, plus the in-flight image which shares almost everything); in round-by-round fixpoint, the peak is quadratic (≈3.3 n², ratio to final ≈ 0.85 n, growing): it is the *peak effect* of the literature, measured. Hamez's SDD library says exactly this and implements saturation **inside** the library (fixpoint h*, auto-detected policy); the hierarchy in bounded classes never lets a global wavefront exist: the intermediate is the current class.
- **But memory and work remain position-indexed.** Even in saturation, a flat engine retains Θ(n²): Θ(n) events × Θ(n) nodes (memo (event, node) ≈ 8 n², union ≈ 7.8 n², table ≈ 7.4 n²), and without GC nothing releases these entries. In naive it is Θ(n³) retained. Hamez's hierarchy reduces the pairs to (event, class): Θ(n); the recursive folding of regular models (Instantiable Petri Nets) shares identical classes and scales the philosophers to several thousand. Neither quotients the pairs themselves.
- **The Lisage target is the quotient of the pairs**: (event class, form, anchor) = 3 × 5 × 1 entries, O(1) + boundary, and a pool bounded to 7 nodes throughout the construction (the discovery sequence of § 6). The table of § 0 says the result there is O(1); this table adds that the intermediate and the working memory are O(1) too, where the best flat-hierarchical engine is Θ(n) live / Θ(n) pairs. This is the milestone 4 protocol: measure SDD/HSDD (nodes, peak, memories) on the same example, the expected baseline being peak ≈ final and near-linear work.

The left-hander changes none of the curves: live peak 70/150/230 (sat, +2 nodes), 316/1 333/3 029 (naive, +1 %), memos +1.2 %. The irregularity costs a constant in all flat policies; on the HSDD side, it breaks the folding of identical modules and requires re-parenthesizing around the seam; on the Lisage side, it replaces S with S' (§ 9) and nothing else.

Sources: the peak effect / internal saturation point is in Hamez, Thierry-Mieg, Kordon, *Building Efficient Model Checkers using Hierarchical Set Decision Diagrams and Automatic Saturation*, Fundamenta Informaticae 94(3-4), 2009, and its ICATPN 2008 version (philosophers benchmark, recursive folding, table 2: 2n philosophers); the regular folding is in Thierry-Mieg, Poitrenaud, Hamez, Kordon, *Hierarchical Set Decision Diagrams and Regular Models*, TACAS 2009. The exact numbers of these tables are to be retrieved for the milestone 4 benchmark (limited HAL access at writing time); the qualities claimed here are those of the abstracts and the extracted text.

---

## 7. Derived operations (one structure, semirings)

- **count** (synthesized, (ℕ,+,×)): the fold runs on forms, not on positions. On line A: a₀(ℓ) = 2a₀(ℓ-1) + a₁(ℓ-1), a₁(ℓ) = 2a₀(ℓ-1) (matrix [[2,1],[2,0]], eigenvalues 1 ± √3); line B is the same matrix with swapped initial conditions (anchor ⊥/⊤); |R_n| = 2a₀(n-1) + a₁(n-1) + 2b₀(n-1), verified exact up to n = 20. Growth (1+√3)^n: counting is a matrix power, O(log n), not O(n). This fold is the expallowedation of D-abs (reduction↔expallowedation duality, (A6) spec § 5.2): the regular run of (t,f) absorbed into a single edge is re-unfolded into a linear recurrence on the inherited, and the regularity of the absorbed buys the exponentiation; counter-test: the left-hander (§ 9.4), same enriched matrix, constant ratio 1.2887.
- **exists** (synthesized, Bool): "can cell 0 eat?": the projection keeps only the (e,m) branch of S: the result is an object of the same pool, barely smaller.
- **member** (inherited): an expallowed state against R_n: we descend with the shift, O(n) on the unfolding, zero storage.
- **deadlock**: empty (hungry always armed from think): filter + exists, one language line.

---

## 8. What the example proves, and what it does not

**Proven here**: the structure of R_n (triple characterization verified), the existence of a 7-node encoding indein-flightent of n, exact counting by fold over the forms. These facts do not dein-flight on the framework: they are the target the framework must reach.

**Bottom-up constructibility is verified too (simulation, November 10, 2026)**: level by level starting from the terminal, with the anchor as a verdict table (4 entries) and the argument on the edge, the construction creates the 4 forms at the first level, then **zero nodes at each subsequent level**, then S at level n: convergence to exactly the 7 nodes, S identical for n = 3..50 (R_5 and R_50 share everything). The same setup in absolute mode (anchor verdicts materialized as nodes, ⊥ impallowed) creates 3 then 4 nodes per level forever: **4n − 4 live, exactly the line of the flat prototype (§ 6.2)**; convergence is therefore a property of the anchor convention (the symbolic anchor), not of the ground, and the boundary of the absolute regime is the same seam as the flat one (elegant detail: at the first level, only B₁ is missing, its boundary node is all-⊥ and reduces to ⊥: the promise "the last one eats" already dead one cell from the end). Controls: table derived from the model (§ 1, § 3) == table of § 4; exhaustive verdicts over the 5^n key words for n ≤ 6; legal states accepted for n ≤ 8; |R_n| by three indein-flightent routes (DP on pc-words, DD paths, fold of § 7) equal up to n = 20 (536 754 176) and n = 50; the 5 states pairwise distinguished by suffixes of length ≤ 2 (C1).

**Not proven, planned**:

1. **Regularity is the best case.** The ring is the terrain chosen by the SDD lineage precisely because it is regular; HSDD already shares storage and work there. The unfold conjecture states that Lisage's flat+shift is the exact unfolding of their hierarchy: the measured comparison (nodes, time, cache) is the milestone 4 benchmark.
2. **The visible added value here**: no hierarchy to build or maintain; the same pool hosts the witness controller (classical BDD for the CUDD differential); the irregularity is absorbed by the boundary: the left-handed philosopher (§ 9) adds **no** interior form, only the root is enriched (7 keys instead of 5) and counting is redone by the same fold; the SDD hierarchy, for its part, must be rebuilt.
3. **The degradation is expected**: arbitrary topology, the forms lose their self-similarity, the representation becomes Θ(n) again then the flat MDD regains the advantage; this is the oracle criterion (§ 1.2 of the proposition): regular repetition → flat+shift; nested irregularity → controller tower.
4. **Design target, partially measured**: the flat prototype validates the structure of the language (sat == BFS, fold == counting), the intermediates (§ 6.2) and the quotient into 5 signatures; but the 7 nodes in pool and the O(1)+boundary remain predictions: the property-based of milestone 1 must verify that saturation converges to exactly these forms (C1, minimality by handle), and milestone 4 must measure them against the hierarchical SDD encoding. If the measurement contradicts the table of § 0, contribution 8 is falsified: that is the protocol.

Method note: the first version of this structure (before simulation) missed the promise carried by the fork of cell 0 and rejected valid states; this is exactly the kind of error the differential test of milestone 1 catches at the first commit. The example was therefore already worth its place.

---

## 9. The irregular variant: the left-handed philosopher

The regular case is the favorable ground of the SDD lineage. Test of the promise "the irregularity is absorbed by the boundary": philosopher 0 seizes in two steps, all others remain atomic.

### 9.1 The model

```
take1_0 : pc_0 = hungry ∧ left_1 = free → pc_0 = g, left_1 = prev   // first step, width 2
take2_0 : pc_0 = g ∧ left_0 = free → pc_0 = eat, left_0 = me        // second step, width 1
release_0 : pc_0 = eat → pc_0 = think, left_0 = free, left_1 = free // as in § 1
philosophers 1..n-1 : model of § 1 unchanged
```

The g state: philosopher 0 holds the fork shared with cell 1 (encoded left_1), waits for the one of its own cell (left_0). Deadlock remains impossible: a single philosopher holds-and-waits, the wait chain cannot close (the others seize atomically). The local type of pc_0 becomes {think, hungry, g, eat}: a cell type changed at one position, not a changed framework.

### 9.2 The reachable language

Circular words of pc, with no adjacent "eat eat", without (pc_0 = g ∧ pc_1 = eat): the left-hander's right-hand neighbor cannot take his held fork. The forks are derived from the pc: left_1 = prev if pc_0 ∈ {eat, g}; left_0 = me if pc_0 = eat; otherwise as in § 3. Verified exhaustively: derivation + these two prohibitions == reachable language (n = 4..7: 72/196/536/1464 states; form == BFS == language).

### 9.3 The structure: still 7 nodes, and the same interior

```
root S' (7 keys, promise on pc_{n-1} as S)
  (t,f)→A₀  (h,f)→A₀  (g,f)→A₁  (e,m)→A₁     // line A : pc_{n-1} ≠ eat
  (t,p)→B₀  (h,p)→B₀  (g,p)→B₁               // line B : pc_{n-1} = eat

A₀, A₁, B₀, B₁ : IDENTICAL to the regular case, anchors included (A₀→⊤, A₁→⊥, B₀→⊥, B₁→⊤)
```

Zero new interior form: the key g exists in no interior form (the left-hander never leaves his position), the irregularity is entirely in S'. Measured on the live nodes of the flat prototype: the interior signatures of the left-hander are exactly those of the regular case (same 4 key sets; one more replica for the boundary signature (t,p)/(h,p), and the root carries 7 keys instead of 5). Total cost: +1 live flat node (4n − 3 instead of 4n − 4), +3 firings, same seam, still 2 passes.

Discovery trace (measured): the first 8 signatures created by the left-hander's saturation are, in order, those of the regular case; the keys (g,f) and (g,p) appear only at ranks 9 and 12. The irregularity grafts onto material already in place, it restructures nothing.

### 9.4 Counting: same fold, richer root

Same matrices as in § 7; the root S' adds an A₁ entry ((g,f)) and a B₁ entry ((g,p)):

> |R'_n| = 2a₀(n-1) + 2a₁(n-1) + 2b₀(n-1) + b₁(n-1), with a₀(ℓ) = 2a₀(ℓ-1) + a₁(ℓ-1), a₁(ℓ) = 2a₀(ℓ-1), a₀(0) = 1, a₁(0) = 0, and b likewise with b₀(0) = 0, b₁(0) = 1.

|R'_4| = 72, |R'_5| = 196, |R'_6| = 536, |R'_7| = 1464 (verified by enumeration), |R'_{20}| = 691 701 760. Ratio |R'_n| / |R_n| = 1.2887, constant over n = 4..20: same base (1+√3)^n, the irregular class shifts the constant, not the exponent.

### 9.5 Flat measurements: the same lesson with the irregularity

| n | 5 | 10 | 20 | 30 |
|---|---|---|---|---|
| final result (live nodes, ⊤ included) | 17 | 37 | 77 | 117 |
| saturation: max image in progress | 16 | 36 | 76 | 116 |
| naive BFS: max image in progress | 31 | 137 | 578 | 1 317 |
| saturation: firings / passes | 48 / 2 | 93 / 2 | 183 / 2 | 273 / 2 |

sat == BFS for n = 4..7, fold == BFS counting up to n = 10 (beyond that, expallowed BFS blows up in memory). At n = 100: 397 live, 78 625 created (regular: 78 325, so +0.4 %); at n = 200: 797 live. The lesson of § 6.2 holds with the irregularity: the naive intermediate image remains quadratic (1 317 vs 117 at n = 30), saturation stays at result + 1, and the irregularity costs a constant.

### 9.6 k left-handers (argument, to be measured)

k irregular positions: identical left-handers share the same root form (same keys), but the handle (controller, anchor, form) distinguishes the positions: O(k) handles; the interior forms remain shared among all philosophers, regular or not. The degradation is linear in the number of irregular points, not in the size of the ring; the SDD hierarchy, for its part, must be redesigned for each new pattern. This is the falsifiable prediction: if milestone 4 measures anything other than O(k), the boundary promise is false.

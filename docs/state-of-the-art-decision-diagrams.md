# State of the art: decision diagram structures

Working document of the project "Generalization and Unification of Decision Diagrams". It extends Linard's thesis, *Parametrizable Semantics of Decision Diagrams: An Approach Towards Unification* (UPMC, 2009), and **polyDD** (Linard, Paviot-Adet, Charron & Buchs, ACSD 2010).

---

## 1. Reading framework

A decision diagram (DD) encodes a function over a product of variables, as a DAG whose internal nodes test a variable and whose leaves carry a value. Almost all existing variants differ along the same axes:

| Axis | Underlying question | Example positions |
|---|---|---|
| Variable domain | Boolean, finite integer, infinite, continuous? | BDD / MDD / DDD / NDD |
| Leaf values | Booleans, integers, reals, functions? | BDD / MTBDD / EVBDD |
| Edge valuation | None, integer, additive, set-based? | BDD / EVBDD / SDD (Couvreur) |
| Hierarchy | Can values themselves be DDs? | HSDD (Hamez), Darwiche's SDD (vtree) |
| Canonicity | Unique form under a given order? | ROBDD, SDD; non-canonical: DDD |
| Operation specification | Hard-coded library vs declarative operators | CUDD vs DDD homomorphisms |
| Exploration strategy | Breadth-first vs saturation | classic BFS vs saturation |

It is precisely this matrix that a unified framework must cover: polyDD tackled it through a **declarative "Controller" mechanism** declaring the order, the edge values, and the applicable operators. Section 9 redoes the exercise exhaustively (November 2026): some fifty families, verified sources, each instantiated in the eight fields of Lisage's extended controller.

---

## 2. Classical line: BDDs and Boolean and pseudo-Boolean variants

- **BDD / ROBDD** — Bryant, 1986. Canonical form under a variable order; the starting point of the whole field.
- **EVBDD** — Lai & Sastry, 1992. Additively valued edges (pseudo-Boolean functions).
- **ZDD (Zero-suppressed)** — Minato, 1993. Removes nodes whose "true" child is empty: optimal for sparse sets (combinatorics).
- **ADD (Algebraic DD)** — Bahar et al., 1993. Leaves with arbitrary values; the basis of symbolic arithmetic.
- **MTBDD** — Fujita, McGeer & Yang, 1997. Multi-terminal leaves for real/integer-valued functions (Markov analysis, PRISM).
- **BMD (Binary Moment Diagrams)** — Bryant & Chen, 1995. Affine edges, for arithmetic verification.
- **Libraries**: CUDD (Somenzi), BuDDy (Lind-Nielsen), and **Sylvan** — van Dijk & Pol, 2016, a **multi-core** framework for BDDs *and* MDDs ([DOI](https://doi.org/10.1007/s10009-016-0433-2)). Sylvan shows that a generic framework can absorb the parallel dimension — an axis absent from polyDD.

**Lesson for unification**: this line is well charted; the "domains" and "leaf values" axes have been systematically explored there. The difficulty of unification begins with operations (each library hard-codes Apply, restrict, etc.) and sharing behind the scenes (libraries share nothing with one another).

---

## 3. Multi-valued line and saturation: the Ciardo/Miner school

- **MDD (Multi-valued DD)** — Miner & Ciardo, 1999, *Efficient Reachability Set Generation and Storage Using Decision Diagrams* ([DOI](https://doi.org/10.1007/3-540-48745-x_2)): variables with arbitrary finite domains, quasi-reduced MDDs, subgraph sharing for Petri net state spaces.
- **Saturation** — developed subsequently by Ciardo, Luttgen, Simnering, Miner (William & Mary): exploring the DD nodes **out of order**, saturating each node with all local transitions before going back up, instead of a layer-by-layer BFS. Exponential gain in practice on state spaces.
- **EV⁺MDD / EVMDD** — additive-valued edges on MDDs, for cost/reward bounds.
- **Reference synthesis**: Ciardo, *Petri Net Analysis Using Decision Diagrams* (QEST 2009, [DOI](https://doi.org/10.1109/qest.2009.14)) — the reference tutorial on the MDD + saturation ecosystem.
- **Meddly** — extensible open-source library grouping MDD, EV⁺MDD and variants; architecturally close to the "framework" spirit.

**Lesson for unification**: there, saturation is a **hand-coded technique** (the saturation order is tied to the model, typically a place hierarchy). One challenge of the revival: making saturation a *declarative property* of the controller, not an ad hoc algorithm — this is exactly the bridge to Hamez's work (§ 4).

---

## 4. The great DDD/SDD/HSDD/ΣDD line: a single family

**Correction of the initial catalogue**: the DDD, SDD/HSDD and ΣDD "branches" are not three independent lines but **a single research family**, the Paris school on decision diagrams (LSV Cachan then LIP6: Couvreur, Thierry-Mieg, Kordon; theses conducted in parallel with those of Hamez and Linard, LIP6/UPMC, 2009) and its Geneva extension (Buchs, Hostettler: ΣDD, MSDD). Read through Lisage's grid, the lineage is not three positions in the parameter space but **a trajectory**: each step named, without knowing it, a constructor of the Σ field. This family surveyed the field before it had a name.

### 4.1 DDD: data, and repetition in embryo

**DDD — Data Decision Diagrams**, Couvreur, Encrenaz, Paviot-Adet, Poitrenaud & Wacrenier, ATPN 2002 ([DOI](https://doi.org/10.1007/3-540-48068-4_8)).

- Variables with **arbitrary (potentially infinite) domains**, edges labeled with data, 0/1 leaves.
- **Non-canonical**: the variable order may vary along paths; this is the counterpart of the flexibility (it allows in-place updates, iteration at variable depth, dynamic domains). Read through Lisage: the repetition of variables, parked as "assumed non-canonicity", is the **list** constructor in embryo ("the repetition of DDDs, tamed", proposition § 1.2).
- **DDD homomorphisms**: the operations are inductive operators (union, intersection, + on variables, *transitive closure fixpoint*) that the user composes; the **libDDD** implementation makes it a declarative library. Historically, this is the first "operational" generalization of DDs: one no longer codes the graph, one codes functions over the graph.

**Direct link with Linard's thesis (2009)**: the parametrizable semantics, instantiated in polyDD, generalizes DDD homomorphisms by separating (i) the graph structure, (ii) edge valuation, (iii) operator semantics — the DDD becoming one instance among others.

---

### 4.2 SDD (Set Decision Diagrams): the set edge (set/map)

- They extend DDDs: the value of a variable in a path is not an atomic datum but a **set** (hence the set-based edge valuation), with sets shared between variables.
- **Hierarchy**: the sets carried by edges can themselves be SDDs over other variables → an SDD can encode **structural regularities** (composed systems, symmetric arrays, well-formed Petri nets) exponentially more compact than any flat DD.
- Implemented in **libDDD** (Couvreur/Thierry-Mieg), at the heart of the **ITS/Stalina** then **NeAT** tools (LIP6).

### 4.3 HSDD and automatic saturation: the nested record (Alexandre Hamez)

Hamez's research program (LIP6/UPMC thesis: *Efficient Generation of Large State Spaces*, 2009):

1. **HSDD + Automatic Saturation** — Hamez, Thierry-Mieg & Kordon, ATPN 2008 ([DOI](https://doi.org/10.1007/978-3-540-68746-7_16)): formal definition of hierarchical SDDs, extension of homomorphisms to the hierarchy, and above all **automatic saturation**: the DDD fixpoint iterator is replaced by a strategy that *dynamically* detects the nodes to saturate, without requiring the user to provide an expallowed variable hierarchy or firing order (cf. Ciardo's "manual" saturation, § 3).
2. **HSDD & Regular Models** — Thierry-Mieg, Poitrenaud, Hamez & Kordon, ATVA 2008 ([DOI](https://doi.org/10.1007/978-3-642-00768-2_1)): use of the hierarchy to encode **regular models** (repeated blocks, symmetries); the structure of the model is mirrored in the structure of the DD.
3. **Journal version** — Hamez, Thierry-Mieg & Kordon, *Building Efficient Model Checkers using Hierarchical Set Decision Diagrams and Automatic Saturation*, Fundamenta Informaticae 2009 ([DOI](https://doi.org/10.3233/fi-2009-137)): the reference presentation.
4. **Synthesis/controllability** — Zhang, Berard, Kordon & Thierry-Mieg, IFAC 2010 ([DOI](https://doi.org/10.3182/20100830-3-de-4013.00047)): beyond reachability, use of HSDDs for **controller synthesis**.
5. **pnmc** — *A Symbolic Model Checker for Petri Nets*, 2016 ([DOI](https://doi.org/10.1007/978-3-662-53401-4_15)): the industrial/tool outcome of this line (open-source symbolic model checker, massively parallel design).

Worth noting: Hamez co-authored *Specification of Decision Diagram Operations* (2010): the bridge between his program and parametrizable semantics already existed, the two theses being conducted in parallel in the same laboratory (LIP6/UPMC, 2009).

### 4.4 ΣDD/MSDD then polyDD: expallowed Σ

The Geneva extension (Buchs, Hostettler): **ΣDD** and **MSDD** (2009), term algebras and multisets, direct precursors of "Σ = the grammar of variables" (proposition, § 1.2). **polyDD** (ACSD 2010) is its culmination: the parametrizable semantics that generalizes the whole family, DDD included.

**Lesson for unification**, in two points:

1. Automatic saturation demonstrates that **the exploration strategy can be computed from the DD structure and the homomorphisms**, not imposed by the model. In a unified framework, "saturation" should become a parametrizable evaluation mode (a fixpoint semantics), of which Ciardo's version (static variable order, § 3) and Hamez's (dynamic detection) are two instances.
2. The family's trajectory reads constructor by constructor: DDD repetition (free order), SDD the set edge (**set**, edge valuation), HSDD hierarchy (**record**, nesting; IPN folding the regularity of **list**), ΣDD the expallowed algebra (Σ itself). Lisage merely names the field that this family explored piece by piece; it is the originating line of the framework, and its most demanding completeness test.

---

## 5. Temporal and numerical lines (mentioned for completeness)

- **CDD (Clock Decision Diagrams)**, **CRD**: clocks and temporal constraints (Larsen et al.).
- **DDD — Difference Decision Diagrams**, Møller & Andersen, ~2002: difference constraints (symbolic DBMs) for timed automata. ⚠️ Name collision with Couvreur's Data DDs — a unified framework could precisely reframe them as different edge valuations.
- **NDD (Numerical DD)** — Wolper & Boigelot: automata accepting Presburger sets, for real/integer arithmetic.
- **IDD (Interval DD)**: interval domains.

---

## 6. The other generalization: knowledge compilation (Darwiche et al.)

A parallel community made the same unification move, but **on the logic side** — not to be confused with the set-based SDDs of § 4.2 (same acronym, different structures).

- **d-DNNF** — Darwiche, 2001: conjunctive decomposition of trees.
- **Knowledge compilation map** — Darwiche & Marquis, 2002 (JAIR): systematic comparison of compiled languages by supported **query class** and **transformation class** — *the* competing paradigm to ours: classifying not by graph structure but by admissible operations.
- **Sentential SDDs** — Darwiche, AAAI 2011: canonical generalization of OBDDs parametrized by a **vtree** (disjunctive decomposition tree); OBDDs are SDDs for degenerate vtrees. Implementation: the SDD package, with dynamic minimization (Choi & Darwiche, AAAI 2013, [DOI](https://doi.org/10.1609/aaai.v27i1.8690)).
- **PSDD** — probabilistic SDD, Kimmig et al., 2014: distributions over SDDs (weighted model counting).
- **Z-SDD** — Choi, Xue & Darwiche, AAAI 2016 ([DOI](https://doi.org/10.1609/aaai.v30i1.10114)): transplanting the zero-suppressed idea into SDDs.
- **Canonicity** — *On the Role of Canonicity in Knowledge Compilation*, AAAI 2015 ([DOI](https://doi.org/10.1609/aaai.v29i1.9423)): what canonicity costs and brings.
- **Recent and relevant**: *Canonical Decision Diagrams Modulo Theories*, 2024 ([DOI](https://doi.org/10.3233/faia241007)) — canonical unification of DDs with background theories; to be read closely as a direct competitor of the revival.

**Lesson for unification**: Darwiche's vtree plays the role of the "controller": a structural parameter on which canonicity and operation complexity depend. A unified framework should be able to express *both* edge valuation (the polyDD axis) and logical decomposition (the vtree axis); the two meet in the question of conditional canonicity.

---

## 7. Existing unification attempts — positioning

| Approach | What it unifies | What it leaves out |
|---|---|---|
| **polyDD** (Linard et al., ACSD 2010, [DOI](https://doi.org/10.1109/acsd.2010.17)) + Linard's 2009 thesis | Parametrizable semantics: orders, domains, values, operators via *Controller* | Native hierarchy, saturation, parallelism, conditional canonicity |
| *Specification of Decision Diagram Operations* (2010) | Declarative specification of operations | Scaling up, implementation |
| DDD homomorphisms (Couvreur et al. 2002) | Operations over arbitrary domains | Classic Boolean structures, canonicity |
| Darwiche's SDD (2011) + KC map (2002) | Compilation languages vs queries | The entire "large graphs/memory sharing" side |
| Sylvan (2016) | BDD/MDD + multi-core | Rich domains, hierarchy, saturation |
| DD modulo theories (2024) | Canonicity + theories | State spaces, hierarchy |
| [MEDDLY / multi-domain package](https://link.springer.com/chapter/10.1007/978-0-387-35190-2_12) (Miner et al.; Becker/Drechsler school) | All MDD types in one package; 11 pseudo-Boolean types (BDD, FDD, OKFDD, MTBDD, EVBDD, BMD, *BMD, p*BMD, HDD, K*BMD, ZDD) | Unification by **framework**, without a controller: each type re-codes its reductions |
| [SLDD](https://www.ijcai.org/Proceedings/05/Papers/0857.pdf) (Wilson, IJCAI 2005) | Query semirings on DD edges | Structure of atoms, attribution (the **construction** semiring is not separated) |

**Clearly open niche**: no existing framework simultaneously unifies (1) generic edge valuation, (2) HSDD hierarchy, (3) saturation as a *declarative* strategy (since Hamez, no one has generalized automatic saturation beyond the Petri/net context), (4) parallelism (Sylvan), and (5) Darwiche-style canonicity guarantees.

---

## 8. Immediate avenues for the revival

1. **Re-read and reformulate** the parametrizable semantics of Linard's thesis (2009) in a single algebraic language (probably: a family of functors over a category of valued graphs — to be explored).
2. **Instantiate the framework** on three benchmark test cases: BDD (canonical), DDD (non-canonical, homomorphisms), HSDD (hierarchical + saturation) — Hamez's automatic saturation as an instance of a parametrized fixpoint operator.
3. **Integrate the Darwiche branch**: prove that the vtree is a special case of controller; derive a conditional canonicity theorem from it.
4. **Prototype**: extend polyDD (or start again from libDDD/Sylvan) with saturation and multi-core controllers.
5. **Experimental validation**: Model Checking Contest as a testbed (the containerized 2018 infrastructure already exists).

---

## 9. The 2026 bestiary, instantiated in Lisage's parameters

Linard's thesis (2009) catalogued the families of the time. The exercise below redoes it as of November 2026, sources verified through bibliographic research (links in table 11), and above all with the grid of the eight fields of the extended controller (proposition, § 1.2):

```
C = ( δ, Σ, decomposition + O, R, default algebra, D^def, encoding, attribution )
attribution = channels (inherited / synthesized / memory) + query semiring + construction merge
```

The **signature** of a family is its position in this parameter space. The classic BDD is the origin of the frame of reference: δ Boolean, Σ = Union(Bool)^n, O total linear, R-collapse + R-share, D^def = ⊥, absolute encoding, inherited only, merge ∨ idempotent. Each family below is characterized by **the field it varies** (the others remaining at the origin). Author attributions are given only when verified.

### 9.1 δ: the atom schema (what keys can be)

| Family | Origin | Lisage signature |
|---|---|---|
| BDD / ROBDD | Bryant, 1986 | The origin of the frame of reference |
| MDD | Miner & Ciardo, ~1999 | δ enumerated; saturation is born there as a static level-by-level policy |
| [LDD (list)](https://mcrl2.org/web/developer_manual/libraries/symbolic/symbolic_library.html) | mCRL2 symbolic library, [Sylvan](https://doi.org/10.1007/s10009-016-0433-2) | δ enumerated, edges = lists of edges: the *list* compilation of Σ; the fusion of adjacent edges ≈ R-collapse |
| DDD | Couvreur et al., 2002 | δ open; free order along paths, R-share only (non-canonical); DDD homomorphisms = inherited only, declarative composition |
| DDD-difference | Møller & Andersen, ~2002 | δ = difference constraints (DBM): the numerical branch of the δ field (cf. the name collision, § 5) |
| [CDD](https://link.springer.com/chapter/10.1007/3-540-48683-6_30) | UPPAAL school, 1998-99 | δ = clock differences, non-convex sets |
| [CRD](https://link.springer.com/chapter/10.1007/0-306-47003-9_15), [CMD](https://www.researchgate.net/publication/221298169) | timed school | δ = constraint matrices: fully structured atoms |
| NDD | Wolper & Boigelot | δ = Presburger encoded by automata; the leaves are automata: the synthesized is rich |
| IDD | cf. § 5 | δ = intervals: the key is a predicate, not a value |

### 9.2 D^def and R: what the absent edge denotes (the most recent lineage)

| Family | Origin | Lisage signature |
|---|---|---|
| ZDD | Minato, 1993 | D^def = **absent** (≠ ⊥): the whole BDD/ZDD difference lies in the default algebra |
| Z-SDD | Choi, Xue & Darwiche, 2016 | Same, on the sentential side |
| CE-BDD | Bryant 1989; Brace, Rudell & Somer ~1990 | The anchor carries a unary transformer (complement) |
| [TBDD, ESRBDD](https://www.cs.utexas.edu/~hunt/FMCAD/FMCAD17/preprints/s5p1.pdf) | FMCAD 2017 | The reduction rule is designated **per edge** (short / X / H0): R becomes an edge parameter, exactly the R field of C |
| [CESRBDD](https://link.springer.com/article/10.1007/s10009-021-00640-0) | Babar, Ciardo & Miner, STTT 2022 | ESR + complement: designated reduction and complement coexist per edge |
| [RexBDD](https://dl.acm.org/doi/10.1145/3649329.3656533) | Ciardo, Miner, Deng & Babar, DAC 2024 | + swap: the three unary transformers of polyDD (reduction, complement, swap) united in a single package |

**Lesson**: the Iowa State 2017-2024 lineage (ESRBDD → CESRBDD → RexBDD) independently converges on "R designated by the controller", one of polyDD's three transformers. A posteriori validation of the R field of C; Lisage absorbs it without extension, and goes further (R-share, R-collapse, carrier default as global controller parameters, not only per edge).

### 9.3 Y, synthesized, merge: the algebraic branch (semirings)

| Family | Origin | Lisage signature |
|---|---|---|
| MTBDD | Fujita et al., 1997 | Y = ℝ: synthesized over (ℝ, +, ×) |
| ADD | Bahar et al., 1993 | Y arbitrary, pointwise apply: the **non-idempotent** merge exists there as early as 1993 |
| [SLDD](https://www.ijcai.org/Proceedings/05/Papers/0857.pdf) | Wilson, IJCAI 2005 | Edges labeled by the semiring: the synthesized-weight of the spec (§ 5), ahead of its time; covers fuzzy, probabilistic, Bayesian CSPs |
| EVBDD | Lai & Sastry, 1992 | Additive affine valuation: the inherited carries (c₀, edge weight) |
| [FEVBDD](https://link.springer.com/article/10.1023/A:1008691605584) | FMSD 1999 | × and + weights on edges: the complement becomes a multiplicative weight |
| BMD, *BMD, p*BMD | Bryant & Chen, 1995; cf. the [multi-domain package](https://link.springer.com/chapter/10.1007/978-0-387-35190-2_12) | Moments: Y = functions, f = c₀ + c₁x: the Davio decomposition is an algebraic merge |
| K*BMD | cf. the multi-domain package | Kronecker × BMD: decomposition type per variable |
| HDD | Clarke, Fujita & Zhao ([CMU chapter](https://www.cs.cmu.edu/~emc/papers/Contributions%20to%20Edited%20Volumes/Multi-Terminal%20Binary%20Decision%20Diagrams%20and%20Hybrid%20Decision%20Diagrams.pdf)) | Hybrid MTBDD × BMD |
| [AADD](https://ssanner.github.io/papers/anu/aadd.pdf) | Tafertshofer & Pedram, 1997 | Algebraic affine: quotient (constant, factor): the anchor carries a pair |
| [EVMDD](https://www.researchgate.net/publication/4248060_Representations_of_Elementary_Functions_Using_Edge-Valued_MDDs) | Nagayama & Sasao, 2007 | Integer weights on MDDs; cf. also EV⁺MDD (§ 3) |
| FDD, KFDD, [OKFDD](https://link.springer.com/article/10.1023/A:1008635324476) | Becker, Drechsler, Sarabi et al., 1994-98 | **Merge = ⊕ of GF(2)** (Reed-Muller): the construction-time fusion changes basis, exactly axis A5 of the spec; the dtl (Shannon / Davio± per variable) is a node law |
| [QMDD](https://arxiv.org/html/2401.01322v1) | 2006 | Y = complex matrices, multiplicative weights on edges |
| QuIDD | 2003, cf. the quantum map | Y = ℂ: ADD restricted to matrices |
| [LIMDD](https://arxiv.org/html/2108.00931v5) | 2022 | Weights = local operators (Pauli): edges typed by a group, Y = stabilizer states |
| PSDD | Kimmig et al., 2014 | Distribution: probabilistic synthesized over SDD |

**Lesson**: almost this whole zoo reads as (Y, semiring of the synthesized, merge). This is the branch that spec § 5/§ 5.1 unifies (the exact fold iff it factors through what the construction preserved). And the "declared construction merge" field remains an **empty cell**: nobody declares their construction ⊕, everyone hard-codes it.

### 9.4 O: order as a constraint, and its liberations

| Family | Origin | Lisage signature |
|---|---|---|
| OBDD | Bryant, 1986 | O total linear: the origin |
| OKFDD (dtl) | Drechsler & Becker | O + node law per variable (Shannon, Davio+, Davio−): the extended Order\<T\> of decomposition laws |
| Sentential SDDs | Darwiche, 2011 | O = vtree (decomposition tree): a constraint, not a sequence |
| dSDD, TSDD | Darwiche lineage | vtree variants |
| free BDD | theory ([Wegener, SIAM 2000](https://epubs.siam.org/doi/10.1137/1.9780898719789.bm)) | O = nothing: canonicity lost |
| IBDD, read-k-times | theory | Bounded repetition of variables along paths |
| [partitioned BDD](https://link.springer.com/article/10.1007/s002240010005) | Bollig, Lobbing & Wegener, 1999 | O = case function: partition of the input |
| parity-rBP | Savický & Sieling | ⊕-nondeterminism |

**Lesson**: typed O (proposition § 3.1) covers OBDD and vtree. Free / IBDD / partitioned live in the non-canonical kingdom (DDD-style): the framework can **express** them (O = relation) but does not promise them minimality (C1). The clean boundary is a contribution, not a failure.

### 9.5 Σ, hierarchy, relative encoding: the structural families

| Family | Origin | Lisage signature |
|---|---|---|
| SDD (set-based) | Couvreur et al. | Edges = shared sets: polyDD's edge valuation |
| HSDD | Hamez et al., 2008-09 | Hierarchy: the edge carries a DD: Σ = nested record |
| IPN, folding | Thierry-Mieg et al., 2009 | Module regularity: *Seq* reused along the hierarchy |
| ESDD | Devereux & Chechik, 2003 | Relative variable on the edge (shift): Lisage's relative encoding, ahead of its time |
| polyDD | Linard et al., 2010 | Generalized edge valuation: the direct ancestor of C |
| ΣDD, MSDD | Geneva group (Buchs, Hostettler) | Term algebra / multisets: Σ as a free algebra, precursor of "Σ = the grammar of variables"; the Geneva extension of the § 4 family |
| MEDDLY | Miner et al. | One package, all MDD types (including EV⁺MDD): unification by framework, **without a controller** |
| [multi-domain package](https://link.springer.com/chapter/10.1007/978-0-387-35190-2_12) | Becker/Drechsler school | 11 types (BDD, FDD, OKFDD, MTBDD, EVBDD, BMD, *BMD, p*BMD, HDD, K*BMD, ZDD) in one package: unification on the formal synthesis side, still without a controller |

**Lesson**: this table cuts by field, but genealogy counts as much as the grid (§ 4): DDD → SDD → HSDD → IPN → ΣDD → polyDD is **a single family**, a continuous trajectory through the Σ field. The catalogue's convergences come from independent teams; the trajectories, from continuous schools. Lisage needs both readings.

### 9.6 The policy: fixpoint and flow

Static saturation (Ciardo, ~2001-2004), automatic saturation (Hamez, 2008), [multi-core on-the-fly saturation](https://link.springer.com/chapter/10.1007/978-3-030-17465-1_4) (Sylvan, 2019), the h* fixpoint of libDDD; and the most recent entry: [Q-Sylvan](https://arxiv.org/html/2508.00514v1) (2025), a parallel quantum package. The thread is continuous, from BDDs to quantum stabilizers, in libraries that each **reinvent their own reductions**: this is the observable cost of the absence of a single controller.

---

## 10. What the catalogue confirms

1. **Every occupied cell is an instance of C.** The eight fields are retroactively validated, and the clearest proofs are the recent convergences: the ESRBDD/CESRBDD/RexBDD lineage (2017-2024) rediscovers "designated R"; SLDD (2005) already had the query semiring; LIMDD (2022) has edges typed by a group (generalized valuation). When independent teams reinvent the same axes fifteen years apart, the axes exist.
2. **The empty cells are the contributions.** (i) Quotient by handle (controller, anchor, form): nobody. (ii) Declared construction merge (A5): nobody separates construction from query. (iii) Full attribution (inherited + synthesized + memory): polyDD alone, and without the channels. (iv) Σ as the grammar of variables (record / union / list / set / map): DDD parks repetition as an anomaly, HSDD captures it in hierarchy, nobody makes it a **type**.
3. **The competing maps.** Darwiche & Marquis (2002) and the [quantum compilation map](https://arxiv.org/html/2401.01322v1) (2024) classify by supported queries; Lisage classifies by controller parameters. The "Typed compilation" line of the spec (operations are defined on types, the controller translates into ordered transductions) is the bridge: a bestiary entry = a signature, a signature = a query class.
4. **The bestiary is not infinite.** Some fifty families, eight axes, and almost all positions (δ, D^def, R, Y, merge, O) already occupied **in isolation**. The remaining space is the cross product (full attribution × grammar × quotient by handle), which is exactly the niche identified in § 7.
5. **One family, one trajectory**: the "DDD branch" of the initial catalogue was not three lines but a single continuous program (§ 4) that discovered the Σ field constructor by constructor. Methodological lesson for the future: cut the bestiary by fields, but always check the genealogies; a school that progresses with each paper on the same axis is proof of the axis's existence, just as much as an inter-team convergence.

**The families that resist (an honest answer).** Three groups, in increasing order of severity:

1. *Outside C1, but expressible*: free BDD, IBDD, read-k-times, parity-rBP. O becomes the trivial (or bounded) relation: the framework codes them, but the quotient by handle (controller, anchor, form) presupposes a determined future; without an order constraint, the same sub-function comes back with different futures, the handle table loses its key, C1 does not apply. The memo (op, node) remains sound (L3: homomorphism in that argument); it is sharing that becomes heuristic. A clean and assumed boundary: the non-canonical kingdom (§ 9.4).
2. *Structural boundary: the coinductive.* The five Σ constructors are all well-founded (μX.(V×X)+1, end marker required). **Regular infinite** values (streams, rational trees) and **cyclic** DDs with fixpoint semantics (well-formed Boolean circuits, recursive equations) would require a cycle node and a quotient by bisimulation, outside C1. (Nuance of November 10: the Lisage pool is cyclic in storage, loops and crossings A₀↔A₁, but its semantics is the unfolding bounded by the anchor, a finite object per n: the empty cell concerns unbounded coinduction, not the fold; see spec § 2.4.5.) This is the only *structural* empty cell of the bestiary: candidate open question no. 10 (the spec already carries question 9 on the continuous).
3. *Boundary of the kingdom: not DDs.* d-DNNF, AIG, circuits: no variable test, decomposition is not an order there. Lisage does not absorb them natively, and should not: the bridge is typed compilation (conversion at predictable cost), not a signature. Likewise undecidable atoms (hybrid, semi-algebraic regions): δ must be a finite decidable key; real encodings (CDD, CRD) are covered, raw continuity is not, by construction.
4. *Size boundary, not coverage*: LIMDD compresses stabilizer states; the arbitrary quantum state remains exponential for **every** DD family (quantum map 2024), nothing to do with the controller. And an unresolved case, not an uncovered one: irregular nested sharing, outside the unfold conjecture; the controller tower (proposition, § tour) is its declared remedy, to be verified at milestone 5.

---

## 11. Verified references

- Couvreur, Encrenaz, Paviot-Adet, Poitrenaud, Wacrenier — *Data Decision Diagrams for Petri Net Analysis*, ATPN 2002. https://doi.org/10.1007/3-540-48068-4_8
- Miner & Ciardo — *Efficient Reachability Set Generation and Storage Using Decision Diagrams*, ATPN 1999. https://doi.org/10.1007/3-540-48745-x_2
- Ciardo — *Petri Net Analysis Using Decision Diagrams*, QEST 2009. https://doi.org/10.1109/qest.2009.14
- Hamez, Thierry-Mieg, Kordon — *Hierarchical Set Decision Diagrams and Automatic Saturation*, ATPN 2008. https://doi.org/10.1007/978-3-540-68746-7_16
- Hamez, Thierry-Mieg, Kordon — *Building Efficient Model Checkers using HSDD and Automatic Saturation*, Fundamenta Informaticae 2009. https://doi.org/10.3233/fi-2009-137
- Thierry-Mieg, Poitrenaud, Hamez, Kordon — *Hierarchical Set Decision Diagrams and Regular Models*, ATVA 2008. https://doi.org/10.1007/978-3-642-00768-2_1
- Zhang, Berard, Kordon, Thierry-Mieg — *Automated Controllability and Synthesis with HSDD*, IFAC 2010. https://doi.org/10.3182/20100830-3-de-4013.00047
- Hamez et al. — *A Symbolic Model Checker for Petri Nets: pnmc*, 2016. https://doi.org/10.1007/978-3-662-53401-4_15
- Linard, Paviot-Adet, Charron, Buchs — *polyDD: Towards a Framework Generalizing Decision Diagrams*, ACSD 2010. https://doi.org/10.1109/acsd.2010.17
- Linard, A. — *Parametrizable Semantics of Decision Diagrams: An Approach Towards Unification*, thesis, Universite Pierre et Marie Curie, 2009 (2009PA066655, theses.fr).
- Linard, Hamez et al. — *Specification of Decision Diagram Operations*, 2010.
- van Dijk & Pol — *Sylvan: multi-core framework for decision diagrams*, 2016. https://doi.org/10.1007/s10009-016-0433-2
- Choi & Darwiche — *Dynamic Minimization of Sentential Decision Diagrams*, AAAI 2013. https://doi.org/10.1609/aaai.v27i1.8690
- Choi, Xue & Darwiche — *Zero-Suppressed Sentential Decision Diagrams*, AAAI 2016. https://doi.org/10.1609/aaai.v30i1.10114
- Darwiche & Marquis — *A Knowledge Compilation Map*, JAIR 2002.
- Darwiche — *On the Role of Canonicity in Knowledge Compilation*, AAAI 2015. https://doi.org/10.1609/aaai.v29i1.9423
- *Canonical Decision Diagrams Modulo Theories*, 2024. https://doi.org/10.3233/faia241007
- Babar & Ciardo — *Tagged BDDs: Combining Reduction Rules from Different Decision Diagram Types*, FMCAD 2017. https://www.cs.utexas.edu/~hunt/FMCAD/FMCAD17/preprints/s5p1.pdf
- Babar, Ciardo & Miner — *CESRBDDs: BDDs with Complemented Edges and Edge-Specified Reductions*, STTT 24, 2022. https://link.springer.com/article/10.1007/s10009-021-00640-0
- Ciardo, Miner, Deng & Babar — *RexBDDs: Reduction-on-Edge Complement-and-Swap Binary Decision Diagrams*, DAC 2024. https://dl.acm.org/doi/10.1145/3649329.3656533
- Wilson — *Decision Diagrams for the Computation of Semiring Valuations*, IJCAI 2005. https://www.ijcai.org/Proceedings/05/Papers/0857.pdf
- Lai, Pedram et al. — *Factored Edge-Valued Binary Decision Diagrams*, FMSD 1999. https://link.springer.com/article/10.1023/A:1008691605584
- Drechsler & Becker — *Efficient Representation and Manipulation of Switching Functions Based on OKFDDs*. https://www.researchgate.net/publication/4225957
- *Implementation of a Multiple-Domain Decision Diagram Package* (BDD, FDD, OKFDD, MTBDD, EVBDD, BMD, *BMD, p*BMD, HDD, K*BMD, ZDD). https://link.springer.com/chapter/10.1007/978-0-387-35190-2_12
- *K*BMDs: A New Data Structure for Verification*. https://www.researchgate.net/publication/3627375_KBMDs_a_new_data_structure_for_verification
- Clarke, Fujita & Zhao — *Multi-Terminal Binary Decision Diagrams and Hybrid Decision Diagrams* (CMU chapter). https://www.cs.cmu.edu/~emc/papers/Contributions%20to%20Edited%20Volumes/Multi-Terminal%20Binary%20Decision%20Diagrams%20and%20Hybrid%20Decision%20Diagrams.pdf
- Tafertshofer & Pedram — *Affine Algebraic Decision Diagrams* (survey AADD, ANU). https://ssanner.github.io/papers/anu/aadd.pdf
- Nagayama & Sasao — *Representations of Elementary Functions Using Edge-Valued MDDs*, 2007. https://www.researchgate.net/publication/4248060_Representations_of_Elementary_Functions_Using_Edge-Valued_MDDs
- Vos et al. — *LIMDD: A Decision Diagram for Simulation of Quantum Computing Including Stabilizer States*, 2022. https://arxiv.org/abs/2108.00931
- *A Knowledge Compilation Map for Quantum Information*, 2024 (taxonomy MTBDD/ADD/QuIDD/SLDD×/QMDD/LIMDD). https://arxiv.org/abs/2401.01322
- *Q-Sylvan: A Parallel Decision Diagram Package for Quantum Computing*, 2025. https://arxiv.org/abs/2508.00514
- mCRL2 — *List Decision Diagrams* (symbolic library). https://mcrl2.org/web/developer_manual/libraries/symbolic/symbolic_library.html
- van Dijk et al. — *Multi-core On-The-Fly Saturation*, 2019. https://link.springer.com/chapter/10.1007/978-3-030-17465-1_4
- Wegener — *Branching Programs and Binary Decision Diagrams*, SIAM 2000 (free BDD, IBDD). https://epubs.siam.org/doi/10.1137/1.9780898719789.bm
- Bollig, Lobbing & Wegener — *An Improved Hierarchy Result for Partitioned BDDs*. https://link.springer.com/article/10.1007/s002240010005
- Bengtsson, Larsen et al. — *Efficient Timed Reachability Analysis Using Clock Difference Diagrams*. https://link.springer.com/chapter/10.1007/3-540-48683-6_30
- Wang — *Symbolic Verification of Complex Real-time Systems with Clock-Restriction Diagram*. https://link.springer.com/chapter/10.1007/0-306-47003-9_15
- *Fully Symbolic Timed Model Checking Using Constraint Matrix Diagrams*. https://www.researchgate.net/publication/221298169

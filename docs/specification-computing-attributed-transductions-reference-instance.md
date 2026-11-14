# Specification: the computation of attributed transductions

*Formalization of the core of Lisage on the reference instance (milestone 1: bounded atoms, `Set`, `Map`). `lookup` is the test operation. Each section ends with what is demonstrable, proven, or left open. Section 2.4 is a discussed extension (generalized defaults, relative forms), outside the scope of milestone 1.*

---

## 1. Scope

Reference instance of the extended controller:

- **Atom schema** δ: *bounded* (finite, enumerable domains, with an optional default).
- **Type algebra**: `Atom(D)`, `Set(T)`, `Map(K, V)` — bounded keys and values. (`Seq`/`Tag`: milestone 2.)
- **Canonicity**: canonical (minimization), with default suppression (sparse representation).
- **Attribution**: the three channels, synthesized over a semiring.

**Posited principle (variable grammar)**: a DD has no variable sequence; its levels unfold an expression of Σ (record, union, list, set, map). The variable repetition of DDDs is the unfolding of list/set/map: repetition of a variable + end marker, a **required key** (never an absorbable default). Level-1 operations are defined on types, **independent of order**, then translated by the controller into ordered transductions. Milestone 1 makes explicit only one compilation: the universe sweep (bounded Set/Map: one presence level per key, δ = absent, no marker); the enumerative compilation (end marker) arrives with `Seq` (milestone 2) and open schemas.

A single example model serves the whole document:

```
K = {a, b}      (canonical order a ≺ b)      V = {0, 1}
S = { {a↦1, b↦0}, {a↦0, b↦0} }              (a set of maps)
```

---

## 2. Decompositions and diagrams

### 2.1 Decomposition

A **decomposition** `Δ` is a tree (or DAG) of *levels*. Each level `ℓ` carries:

- `Val(ℓ)`: finite domain of edges (decision values);
- `Δ(ℓ, v)`: successor sub-decomposition of edge `v`;
- `δ(ℓ) ∈ Val(ℓ) ∪ {⊥}`: **default value** (removable edge), parameterized by the controller's default algebra.

For bounded `Map(K, V)` (canonical order `k₁ ≺ … ≺ kₙ`):

```
k_i :  presence ∈ {absent, present}   δ(k_i) = absent
         └─ if present : val(k_i) ∈ V   (atomic level, δ = ⊥)
```

The *presence* level is exactly the set valuation of SDDs; `δ = absent` is the sparse convention (a direct generalization of the ZDD).

### 2.2 Semantic diagrams

```
D ::= ε(y)                    payload terminal y ∈ Y
    | ν(ℓ, {v₁ ↦ D₁, …, vₘ ↦ Dₘ})     level-ℓ node
```

The edges absent from a node mean `δ(ℓ)` (default suppression). **Set semantics** (the A/B interpretation, set or function, settled here):

```
⟦ε(y)⟧        = {( [], y )}                          an empty path with payload y
⟦ν(ℓ, A)⟧    = ⋃_{v ↦ D ∈ A} { ( [v·w], y ) | (w, y) ∈ ⟦D⟧ }
              ∪  { ( [δ(ℓ)·w], y ) | (w,y) ∈ ⟦D_δ⟧ }   if the default is "carrying"
```

The "carrying default" case (an absent edge continues the path with `δ(ℓ)`) characterizes presence: an absent key remains absent throughout the rest. A DD therefore denotes a **set of paths with payload**: the function interpretation (`X → Y`) is the case where each assignment prefix admits at most one payload; the set interpretation (`Y = Bool`) is the trivial functional case. *The two readings are two well-typing conditions, not two objects.*

### 2.3 Reductions = quotients

With `Δ` fixed, `D` denotes a partial function (prefixes → payloads). Two diagrams are **equivalent** (`≡`) if they denote the same function. The reduction rules are simplifications preserving `≡`:

```
(R-collapse)    ν(ℓ, {v ↦ D | v ∈ Val(ℓ)})  →  D          (all edges lead to the same D)
(R-default)     ν(ℓ, A ⊎ {δ(ℓ) ↦ D})  →  ν(ℓ, A)          if D ≡ D_δ ("carrying the default")
(R-share)       identical sub-DDs ⇒ same node (interning, structural)
```

`Canon(D)` = minimal form modulo these rules — the minimal coalgebra of § 1.3 of the proposal. On the example:

```
S canonical:
        k_a?
        └─ present
           └─  [R-collapse on val(a): 0 and 1 lead to the same sub-DD]
              k_b?
              └─ present
                 └─ val(b)=0 ↦ ε(⊤)
```

**Decisive observation for what follows**: after `R-collapse`, the level `val(a)` no longer appears — the value of `a` has become *implicit* (any value will do). The representation has lost information it did not need to keep *as a set*… but that some operations will have to recover (§ 6).

### 2.4 Extension: generalized defaults and relative forms (milestones 4–5)

Two generalizations of the system of 2.3, discussed after it was written. They extend the meta-theory without modifying it: milestone 1 implements 2.3; the extension is for milestones 4–5, behind the oracle.

#### 2.4.1 Generalized default (D-abs)

(R-default) only absorbs the edge labeled `δ(ℓ)`. Generalization: the controller designates a **default diagram** `D^def` (a canonical form designated *by controller*, never by level, never a free sub-DD: otherwise the semantics of a node could not be recovered from (form, controller)). Any key whose target ≡ `D^def` is then absorbable, on **any sub-domain**:

```
(D-abs)    ν(ℓ, A ⊎ {v ↦ D^def | v ∈ R})  →  ν(ℓ, A)     for all R ⊆ Val(ℓ)
(D-trans)  ν(ℓ, A) with A = ∅ or all targets ≡ D  →  D   transparent node, composed offsets
(R-share)  equal forms ⇒ same node                       unchanged, structural
```

(R-collapse) and (R-default) become special cases; the instance is chosen in the default algebra:

| Instance | Designated `D^def` | Effect |
|---|---|---|
| BDD (don't care) | the jump-continuation δ | classical absorption; D-trans = collapse |
| ZDD (presence) | the "empty" presence diagram | zero-suppression |
| MTBDD | the neutral terminal | absorption of null edges |

Limit case: "the rest ↦ D₂ ≠ D^def" (MDD with a large domain) is not compressible by D-abs alone; it is if the controller designates D₂ as the default for that application. The designation is a parameter of the controller, not of the form.

#### 2.4.2 Relative forms: the variable leaves the node

Second extension: the variable identity can leave the node for the predecessor edge.

```
Relative form:  ν( K )          K : finite keys, v ↦ (offset_v, sub-form_v)
handle = (form, anchor)           anchor = absolute variable of the root
ℓ(n) = ℓ(parent) + offset          the offset accumulates in the inherited channel (φ_disp)
```

- **Polymorphic encoding**: each controller decides whether its nodes carry the variable (absolute), receive it from the edge (relative), or mix the two with a controller-fixed **precedence**; an absolute node reached by a relative edge resynchronizes the accumulation (resynchronization point, useful for events with non-uniform span).
- **Typing by inclusion**: placing a form s on variable x is valid iff `keys(s) ⊆ Val(x)`; the complement is covered by the absent edge. Sharing between variables with different domains is sound under **inclusion**, not under equality.
- **Inverse reductions**: re-explicating a removed variable (quantification, named substitution) = T-EXPAND (§ 6.2), available because operations traverse top-down, accumulate, and are defined on a controller that knows the order.

#### 2.4.3 Identity: the triple (controller, anchor, form)

The low-level pool is a **canonical storage with no other knowledge**: several controllers coexist; two forms at the same level may result from two different reductions carried by two distinct controllers; the reading of a form is fixed by whoever traverses it.

> **Canonicity migrates from the node to the handle.** Counter-example: s = {0 ↦ D, 1 ↦ D} with D ≢ `D^def` is collapsable on a Boolean variable (its canonical form there is D alone) and irreducible on a three-valued variable (the absent edge there covers 2 ↦ `D^def`). The classical invariant "every stored node is reduced" disappears; it becomes: every handle is canonical for its reading (controller, anchor). Consequences: reduction applies at handle creation (the anchor is known there); the handle table is keyed on (form reduced for that reading, anchor); conversions between controller kingdoms = transductions with predictable cost (the typed contract extends from orders to encodings).

> **(C1, to be proven)** Confluence of the system {D-abs, D-trans, R-share}, then minimality per handle. The format of T5 extends: canonicity is relative to the contract (controller, order, anchor), never a property of the bare structure.

- **Caches**: the computed-table keys carry the controller's fingerprint (at minimum its default algebra). In a homogeneous regime, equivariant operations are keyed on the anchor difference; in a heterogeneous regime, anchors come back into the keys.
- **Saturation**: it iterates over *uses* (form × anchor pairs), not over forms; the level → used-forms index lives in the shadow plane. Storage is shared, traversal is not (except below).

#### 2.4.4 What the extension opens up

- **Work sharing (falsifiable)**: on N replicated components with translation-invariant events, the computed cache deduplicates work between replicas, not just storage. Statement: *O(1) cost for the core of the replicas, plus a boundary term* (sewing a ring breaks the invariance). Benchmark: n philosophers, baseline: the hierarchical SDD encoding.
- **Flat what the hierarchy does in SDD (conjecture)**: the flat shift is the unfold of a *regular* hierarchy; if the decomposition is uniform, the flat equals the vtree with fewer machines; if it is irregular, it loses. This is where the completeness test (sentential SDDs without ad hoc extension) resurfaces: the relative instance may be the missing brick for installing SDDs in the flat world.
- **Lineage**: ESDD ([Devereux & Chechik 2003](https://www.oldcitypublishing.com/journals/mvlsc-home/mvlsc-issue-contents/mvlsc-volume-9-number-1-2003/mvlsc-9-1-p-59-86/)): multi-valued, structure sharing, canonicity via sufficient conditions on unary edge transformers; pre-saturation, never adopted. CESRBDD ([Babar/Ciardo/Miner, STTT 2022](https://link.springer.com/article/10.1007/s10009-021-00640-0)) then RexBDD ([Ciardo/Miner/Deng/Babar, DAC 2024](https://par.nsf.gov/biblio/10527627-rexbdds-reduction-edge-complement-swap-binary-decision-diagrams)): reduction on the edge, complement and swap. Lisage's contribution: unification (every unary edge transformer = a structural valuation, an instance of the controller) and integration (typing by inclusion, canonicity per handle, T-EXPAND, oracle).

#### 2.4.5 Two more shifts: composed anchors, relative values (November 10, 2026)

One idea, two group actions: **the form is relative, the context carries the absolute**. The positional anchor (2.4.2, 2.4.3) is the case where the group acts on indices; two extensions complete the symmetry.

- **Composed anchors (named shifts)**: when the network factorizes into several distinct fragments (different periods, interleaved in the order), the anchor becomes a **named record** of shifts, one per fragment ({machine: +i, conveyor: +j}). Forms factorize per fragment (small composed forms, instead of the word enumerated over the lcm of the periods); the degradation stays at O(Σ kᵢ) irregular points, never O(∏) (falsifiable: a pair of independent irregularities must not cost the product); seams are counted per fragment boundary. Canonicity: lexicographic order of names, zero = start of fragment; the anchor stays in the handle (C1 unchanged), the pool stays blind. § 9.6 of the philosophers document (k left-handers: O(k) handles, shared forms) is the single-fragment special case; the controller tower remains reserved for the irregularly nested; fragment decomposition is a decision of the oracle.
- **Key transformations carried by the controller**: nothing specific to the shift lives on the edge, and nothing lives there at all that is not a **term of the key grammar**. The controller designates, as for everything else, a **group G of transformations** acting on the key domain; forms are normalized modulo G by a **canonical section of the orbits** (chosen by the controller); the **function** lives in the controller, **the incoming edge carries the argument**: for a shift, the effective value of n (the minimum of the true keys in the min = 0 section); the edge's term is f(n), the edge remains (term, handle), the pool stays blind, and hash-consing is keyed on the reduced form as early as M1-1. The precedent is already in the philosophers document (§ 4): the shift y lives on the edge, +1 per cell. The argument participates in the hash of the **parent**'s form (as term data), never interpreted by the pool. The +n shift is the instance (G = (ℤ,+), section: the smallest edge labeled 0, the reconstruction term on the incoming edge); the complemented edge is another (G = Z₂, section: the terminal convention); the RexBDD swap a third (action on the children, same rank: the attribute lives on the edge). Added algebra contract (it extends that of region-keys): (i) the action is **decidable and invertible** (a group: absolute reading and product require t⁻¹, renormalization t_B∘t_A⁻¹, O(1) terms per pair); (ii) the transformations are **bijections** (D^def invariant: complement commutes, "everywhere else" does not change); (iii) the canonical section is **confluent** (normal form, otherwise no hash-consing); (iv) admissibility (3.3) requires that the operation commute with the action or carry its relative parameters, and the memo (L3) is keyed without the term for invariant operations; (v) the family f is **closed under composition** with a law computable on the arguments: f(n₁)∘f(n₂) = f(n₁ ⊕ n₂), a homomorphic embedding of (A, ⊕) into G (the accumulation of the absolute reading happens on the arguments, O(1) per step); (vi) the argument is stored **modulo the kernel of f**: f(n) = f(n') ⇒ same canonical writing of n and n', otherwise two spellings of the same transformation split semantically identical parents and C1 is violated relative to the key algebra. Reading: effective key = f(n)(stored key); the invariant of Q6 extends: absolute reading = composition of the arguments (n₁ ⊕ n₂ ⊕ …). Applications: unbounded counters ((ℤ,+)), color classes (permutations), convex regions (translation of (ℝᵈ,+): the first fragment δ continuous, cheap); DBM closure is **not** of this form (closure operator, not an orbit section) and keeps its own treatment. Living instance verified on November 10: the sliding window (canvas second-golden-thread-example-the-sliding-window-protocol): the sequence base = the argument on the input edge, the composed anchor {base, horizon} designates the regulated run, and Wireshark makes the canonical section min = 0 by default (relative sequence numbers): the industrial precedent of clause (vi). Verified cousins (state of the art § 11): complemented edges (Z₂, terminal side), EVBDD/FEVBDD/AADD (additive, terminal side), DDD (difference constraints, canonicity by closure); the case (group designated by the controller, key side, local section) is not in the bestiary: to be verified as a claim, not presumed.
- **Composition**: the two coexist, the anchor (position) in the handle, the transformation term (value) in the key grammar; the philosophers do not need it (symbolic keys), counters and clocks do. As always: on the algebra/ side, zero change to the pool.
- **Variable increment: a function of the controller, parameterized by the edge, as for values** (precision of November 10). Nothing freezes the step: incrementing a variable (the anchor shift of a level, +1 per cell; the effect of an event on a counter, a clock, the sequence base) is applying **a transformation designated by the controller whose parameter lives on the edge**, exactly the mechanism of key transformations. The relative writing (x ↦ x ⊕ δ) is the general case, the absolute writing (x ↦ v) the degenerate case (a constant transformation); both are terms of the same grammar, and clauses (v)-(vi) cover the increment without modification: composition of increments = accumulation of arguments (O(1) per step, the invariant of Q6), storage modulo the kernel. Instance: the slide of the sliding window advances the base by f(δ), δ carried by the event; the verified trace only exhibits δ = 1, but nothing in the setup requires a constant step: an acknowledgment of r cells at once is the same term f(r). Unification: the anchor (position), the key transformation (reading) and the variable increment (effect) are **three uses of a single mechanism** (the function in the controller, the argument on the edge, the blind pool); the pool learns nothing new.
- **The root edge: the value of a DD is an edge**. Form normalization deprives the root of its incoming edge; the root's argument therefore lives on a **virtual input edge**, and the type "standalone DD" is the pair **(term f(n), handle)**, not the bare node. Consequences: (i) operations take and return edges (transductions sign edge → edge), the descent starts with the input argument, and the absolute-reading invariant starts there; (ii) the memo is keyed on the form alone for invariant operations (the argument is already normalized by construction), and on (form, argument) otherwise; (iii) two DDs sharing all their nodes but differing in input are two distinct DDs: R₅ and R₅₀ share the 7 nodes, the input carries the parameter; (iv) Liseuse renders the input edge and its term, it is the visual identity of the diagram. The rank is standard in edge-attribute families: CUDD's root pointer carries its complement bit, an EVBDD's root carries its offset; the value of an attributed DD is an edge, never a bare node. None of this materializes a node: the pair (term, handle) is carried by the caller (algebra/, plan/), the pool remains unchanged.
- **The DD remains a DAG, the pool does not**. The pool is cyclic in storage, and always has been: loop (t,f) on A₀, crossed edges A₀↔A₁ (the philosophers' fold, the minimal automaton of the language of legal cells); the "DAG" of level 2.1 is the semantics, not the pool. At fixed n, the unfolding traverses n cells then takes the anchor edge to the terminals: acyclic by construction. The pool is therefore a **finite presentation of an infinite family of DAGs** (one per n), not an infinite object: the unfold conjecture (2.4.4) in action. This is also why this cycle does not occupy the "cyclic" slot of the bestiary (state of the art, open question 10: cyclic DDs with fixpoint semantics describe an infinite object and require a cycle node + a quotient by bisimulation): here the fixpoint is **never taken**, the anchor bounds the unfolding, C1 holds without bisimulation. Termination consequence: it no longer comes from the acycallowedy of the pool but from the **well-order on the accumulated argument** (shift +1 per cell, the anchor is the bound); the descent has bounded depth on a cyclic graph, the memo is keyed on (form, argument). And the input edge is the point where n enters the cycle: without it, the A₀ loop would be unreadable (infinite rotation or fixpoint). Terminology to maintain: "DD" designates the unfolded (DAG), "pool" the folded (cyclic graph); the input is an edge in the pool, the value a root of the unfolded DAG. Verified on the philosophers (simulation, November 10): the bottom-up construction level by level from the terminal converges to exactly the 7 nodes (the 4 forms at the first level, zero creation afterwards, S at level n), because the anchor there is a table of verdicts and the argument lives on the edge; the same setup in absolute, verdicts materialized as nodes, creates 4 nodes per level indefinitely (4n − 4 alive, the line of the flat prototype): the convergence is an effect of the anchor convention, not of the terrain. And the execution **never unfolds**: the descent interprets the cyclic pool on **(form, argument) pairs**, the memo guarantees that a pair is paid once, and the cost of an operation is the **number of handles visited, never the number of paths** (the combinatorial explosion lives in the paths: 2ⁿ for the philosophers; the unfolded canonical DD has only Θ(n) handles; the pool, O(1) forms). Two regimes (clause iv): operations **invariant** in the argument (verdicts, guards) → memo on the form alone; non-invariant operations (folds, counts) → no table on the argument but an **accumulator**: the composition of clause (v) keeps O(1) state per level, |R₅| = 536 754 176 is counted in O(n) (and in O(log n) by exponentiation of the fold, the pool being a cycle with constant transition), never in O(|R_n|). The memo is **transient** (per call), the pool **persistent**: it is the one that stays O(1) while the argument lives its life (the window base traverses ℕ, the pool stays 4). Avowed worst case: if the unfolded canonical DD is exponential (a model without regularity), the memo is too; C1 is minimality, not magic, and the promise is to **never pay more than the canonical**, the family's storage remaining shared.

---

## 3. The computation of attributed transductions

### 3.1 Definition

An **attributed transduction** `τ` (for a fixed controller) is given by:

- a set of **inherited attributes** `H`, with an initial element `h₀`;
- a set of **synthesized attributes** `S`, equipped with an aggregation structure;
- a **level memory** `M : (id(node), ℓ) ⇀ value`, with buffers;
- three partial functions of the controller:

```
φ_disp :  (ℓ, h, v)  ↦  h'            dispatch: inherited attribute per edge
φ_aggr :  (ℓ, h, {v ↦ s})  ↦  s       synthesis: attribute rising from the node
φ_out  :  (ℓ, h, {v ↦ (D', s)}, M)  ↦  (result node, memory effect)
```

and two mutually recursive execution rules:

```
(T-TERM)   τ(ε(y), h) = ( ε(ψ_y(y, h)) ,  σ_y(y, h) )

(T-NODE)   n = ν(ℓ, A)
           hᵢ = φ_disp(ℓ, h, vᵢ)              for each edge vᵢ ↦ Dᵢ ∈ A
           (Dᵢ', sᵢ) = τ(Dᵢ, hᵢ)             recursion (descending)
           s  = φ_aggr(ℓ, h, {vᵢ ↦ sᵢ})       synthesis (rising)
           (n', μ) = φ_out(ℓ, h, {vᵢ ↦ (Dᵢ', sᵢ)}, M)   restructuring
           ─────────────────────────────────────────
           τ(n, h) = (n', s)                 [with M ← M ⊕ μ]
```

`φ_out` is the only rule that *writes* the diagram: it can filter edges, relabel, **change level** (the order is not fixed by the structure — this is where the order/operations entanglement is embodied: a transduction can reorder its output), skip a level (re-emit a collapse) or **expand an implicit level** (§ 6.2).

### 3.2 Typing

```
τ : DD⟨Δ_in, Y_in⟩ × H  →  DD⟨Δ_out, Y_out⟩ × S
```

`Δ_out` is determined by `τ` and *may differ from* `Δ_in` (transductions changing order or codomain). Semantic correctness is a **specification obligation** attached to `τ`:

> **(Spec-τ)** For each `τ`, provide `f_τ : ⟦·⟧ × H → ⟦·⟧` such that
> `⟦π₁(τ(D, h))⟧ = f_τ(⟦D⟧, h)` — the transduction implements its semantic function.

The proof of `(Spec-τ)` is by induction on the `T-TERM`/`T-NODE` rules, case by case. This is the *format of all correctness theorems of the framework*: each operation instance brings its semantic function and its induction.

### 3.3 Admissibility (compatibility with canonicity)

A transduction is **admissible** if:

- **(A1-default)** `φ_disp` handles absent edges via `δ(ℓ)`: no implicit information is ignored without being reconstructed;
- **(A2-collapse)** if all edges of an input node produce the same `(D', s)`, then `φ_out` re-emits a collapsed node (the result is no more explicit than necessary);
- **(A3-canon)** `φ_out` emits only interned nodes (the result is canonical by construction, not by post-processing).

> **Lemma L1 (memoization).** Under (A1)–(A3), `τ(n, h)` is a pure function of `(id(n), h, version(M))`. *Proof*: the rules read only the sub-diagram of `n` and the memory; the version of `M` falsifies earlier entries. **Consequence**: computed table keyed on `(id(n), h)` — the generalization of Bryant's `(operand, operand)` and of the `(h, D)` of DDD homomorphisms.

> **Lemma L2 (commutation).** If `D ≡ D'` and `τ` is admissible, then `τ(D, h) ≡ τ(D', h)`. *Proof*: induction on the derivation of `≡`; critical cases: collapse (by (A2)) and default (by (A1)). **Consequence**: operations can work on any physically equivalent form — the representation plane is free.

---

## 4. First family: inherited-only transductions

`map`, `filter`, relabelings: `S` trivial, memory unused.

**Example — `filter p` on `Set(Map(K,V))`** with `p(m) = (m(a) = 1 ∨ ¬present(a))`:

```
φ_disp(k_a?, h, present)  = h_seek_value_a          (descend with intent)
φ_disp(val(a), h, v)      = v = 1 ? garde : tue         (the verdict is resolved here)
φ_out : rewrites the edges according to the verdicts coming back up
```

Note: the verdict is resolved *at the value level*, but applied *at the presence level* — without a rising channel, the classical DDD implementation must either enumerate the paths or reify the verdict in a variable. Here `φ_aggr` carries the verdict upward in one line. *(Spec-τ)*: `f_filter = preimage of p` — direct induction.

---

## 5. Second family: synthesized-only (semirings)

`count`, `exists`, `fold`: `H` trivial, the terminal produces the base, `φ_aggr` the semiring structure.

```
count  :  S = (ℕ, +, ×)        σ_y(⊤, h) = 1        φ_aggr = Σ of the edges (× by the subtotals)
exists :  S = (Bool, ∨, ∧)     σ_y(⊤, h) = vrai    φ_aggr = ∨ of the edges
```

**Counting on the example**: the *explicited* paths of canonical `S` number 1 — and yet `|S| = 2`. The rule:

> **(A4-expansion-counting)** for a collapsed level `ν(ℓ, {v ↦ D | v ∈ Val(ℓ)})`, the weight of the implicit edge is `|Val(ℓ)| × weight(D)` — the cardinality of the level enters the aggregation.

`count(S) = |{0,1}| × 1 (k_a present) × 1 × 1 = 2`. ✔ **The collapse forces the semiring to know the cardinalities of the levels** — this is the first appearance of the explanation principle (§ 6.2), on the aggregation side: *implicit information must be counted, not ignored*. `(Spec-τ)`: `f_count = cardinality of the denoted set` — the proof requires (A4).

### 5.1 Where does the "+" live: three roles, two algebras

The symbol ⊕ appears in three places that do not live at the same level:

| Role | Site | Declaration |
|---|---|---|
| (a) fusion at construction | the ⊕ of `saturate` (§ 7): combining two diagrams in writing | **implicit today**: to be made of the instance (§ 7 amended) |
| (b) fold aggregation | `φ_aggr`: combining a node's children on the way up | semiring of the transduction (this § 5) |
| (c) pointwise denotation | what the diagram denotes when `Y ≠ Bool` | the typing `DD⟨X,Y⟩` (MTBDD/ADD, instance table of the proposal) |

> **(A5-two-algebras)** The **construction** semiring (the merge of writings) and the **query** semiring (parameter of the transduction) are two distinct declarations, both of the instance. Today the construction is idempotent `Bool`, and **hash-consing is this choice**: idempotent union destroys multiplicity at writing time. A fold is **exact iff its reading factors through what the construction preserved**: cardinal `count` is exact on a deterministic canonical diagram (function interpretation, § 2.2: one path per assignment); any weighted fold (probabilities, path multipallowedies) is unrecoverable on a Bool construction and requires a **lifted construction** (weight-carrying edges, pointwise ⊕ merge in the ADD style), where additivity is restored by distributivity at the price of idempotence (no more collapsing of duplicates).

> **(L3-memo)** The memo (operation, node) is sound ⟺ the operation is homomorphic in that argument. Consequences: semantically additive operations (union, image by a relation, guard = restriction) memoize per node; **a fold is terminal, never incremental, unless ⊕ is idempotent**: ∃-summaries (`exists`, verdicts) are maintained during saturation, cardinalities fold at the fixpoint. The semiring axioms are exactly the license for `f(d₁ ⊕ d₂) = f(d₁) ⊕ f(d₂)`: `count` is homomorphic for the structure (a memoizable fold, cf. (A4)) and non-additive for the denotation (∪ idempotent); correctness requires naming the two algebras, not choosing between them.

### 5.2 Every reduction has an evaluation dual

> **(A6-duality)** Every reduction (the syntax forgets) has an evaluation dual (the reading re-explicits, and pays). **The cost of a fold = the cost of re-expliciting what the reductions absorbed:**

| Reduction (syntactic forgetting) | Evaluation dual (paid re-explanation) |
|---|---|
| R-share | memoization of the fold by (form, inherited): sharing pays the fold once |
| R-collapse | (A4): the cardinal `|Val(ℓ)|` enters the aggregation |
| R-default / carrying default | the fold adds the weight of the implicit δ continuations (the absent edge continues the path: its contribution is counted) |
| D-abs (default run absorbed into one edge) | the fold's recurrence on the inherited (the length re-unfolded): matrices, exponentiation |

Falsifiable prediction: the regularity of the absorbed buys fast exponentiation (the philosophers' fold: matrix [[2,1],[2,0]], O(log n) instead of O(n)); the irregularly nested pays the explanation at full unfolding, Θ(n) or worse. Measured counter-test: the left-handed philosopher (example canvas, § 9.4), same enriched matrix, ratio |R'_n| / |R_n| = 1,2887 constant: the irregular class shifts the constant, not the exponent. This is the link to counting by handles: **the philosophers' fold is the explanation of D-abs** (example canvas, § 7).

---

## 6. The test operation: `lookup`

### 6.1 Specification

```
lookup(k) : DD⟨Δ_map, Bool⟩ → DD⟨Δ_map, Option(V)⟩
⟦lookup(k)(D)⟧ = { ([m], m(k)) | ([m], ⊤) ∈ ⟦D⟧ }
```

The result is a *functional* DD: each map of the set receives at the terminal the value of key `k`. On the example: `m₁ ↦ Some(1)`, `m₂ ↦ Some(0)`.

### 6.2 The central difficulty: implicit levels

On **canonical** `S`, the level `val(a)` has disappeared (collapse) — yet the result is *not* constant in `a`. A naively recursive transduction reads `ν(k_a?, …)` then finds `k_b?` without ever seeing the value of `a`. **Structural conclusion**:

> **Expallowedation principle.** Any operation non-constant on an axis must explicate the implicit levels of that axis; this is possible **iff the universe of the axis is enumerable** (bounded schema). Expallowedation is a rule of `φ_out`:
>
> ```
> (T-EXPAND)  n' absent from the sought axis ⇒ n' = ν(val(a), { v ↦ D_v | v ∈ Val(val(a)) })
>              with D_v = copy of the sub-diagram (the axis becomes explicit again)
> ```

This is **the schema-transduction correspondence theorem**:

> **T5 (correspondence).** Bounded schema ⟺ every level-1 operation compiles into a transduction (with expansion) ⟺ usable canonicity. Open schema ⟹ expansion is impossible ⟹ the decomposition must enumerate the present elements (DDD style) ⟹ assumed non-canonicity. *The canonicity classification of § 2 of the proposal is not a property of the representation: it is forced by the computation of operations.*

### 6.3 The definition of `lookup(a)`

Two inherited modes: `seek` (above the axis), `extract` (in the axis). Synthesized attribute: **a DD fragment** on the axis — synthesized attributes are *second-order* (see § 8).

```
φ_disp(k_a?, seek, present)  = extract
φ_disp(k_b?, seek, ·)        = seek        (the other keys: transit)
φ_disp(val(a), extract, v)      = stop           the mode becomes terminal

(T-TERM in stop mode)     σ_y(y, stop)      = ε(Some(v))     fragmentDD = payload
(T-TERM in extract mode)  σ_y(y, extract)   = —              (unreachable: val(a) comes first)
(T-TERM in seek mode)  σ_y(y, seek)    = ∅              not found yet

φ_aggr(ℓ, seek, {vᵢ ↦ sᵢ}) = union of the fragments (canonical merge of the fragment DDs)
φ_aggr(val(a), extract, {v ↦ s}) = ν(val(a), {v ↦ s})      re-roots the fragment

φ_out(k_a?, seek, {present ↦ (D', s)}, M) :
      D''  = graft: the terminals of D' receive the payload of fragment s
             (the fragment is "applied" to each continuation)
      n'   = ν(k_a?, { present ↦ D'' })
```

The **graft** (`φ_out` of the sought level) is the operation that existed in no previous formalism: the synthesized fragment — harvested at the bottom of the axis — is *distributed over the terminals of the rest of the diagram*. This is exactly "carrying inter-level information": neither purely inherited (the fragment does not descend), nor purely synthesized (it does not rise far enough), but **both, coordinated by the axis node**.

### 6.4 Derivation on the example

```
Input (canonical):  k_a? ─ present ↘ [val(a) implicit]  k_b? ─ present ↘ val(b)=0 ↘ ε(⊤)

1. τ(k_a?-node, seek): dispatch present ↦ extract; other default (absent ↦ ∅)
2. under the present edge: next level = k_b?  ⟹ level val(a) IMPLICIT
   (T-EXPAND)  :  ν(val(a), { 0 ↦ Q, 1 ↦ Q })   with Q = copy of the k_b? sub-DD
3. τ(Q, seek) on each branch: transit k_b?, count-like:
   σ brings the fragment ∅ back up (no a in the branch)… no — the branch IS under val(a):
   in "extract" mode, the base is set by (T-TERM in stop mode) — but we do not
   reach val(a) yet. Hence the correction: the extract mode applies to the LEVEL val(a) itself;
   its edges set σ = ε(Some(v)); the graft is therefore ν(val(a), {0 ↦ G, 1 ↦ G'}) where
   G, G' carry Some(0), Some(1).
4. φ_aggr(k_a?) brings the graft back up; φ_out grafts:
   k_a? ─ present ↘ ν(val(a), { 0 ↦ [k_b?… ε(Some(0))], 1 ↦ [k_b?… ε(Some(1))] })
```

Result: function `m₁ ↦ Some(1)`, `m₂ ↦ Some(0)`. ✔ The result is not collapsable at `val(a)` (the two edges lead to *different* sub-DDs): **the operation has explicated the axis** — exactly what the semantics required, and `R-collapse` no longer applies. The derivation and the reduction *agree*: this is the sign that the computation is correct.

---

## 7. Third family: `saturate` (level memory)

```
saturate(τ, D)  =  νX. D ⊕ τ(X, h₀)        X = the current set; ⊕ = construction merge,
                                           declared by the instance (A5): canonical union,
                                           idempotent (Bool) today; in weighted reading,
                                           pointwise ⊕ in the ADD style: additivity by distributivity,
                                           loss of idempotence
```

`τ` here is a step transduction (the image of one step of the system). The **level memory** `M` lives between iterations (key: `(id(node), ℓ)`): it carries the intermediate state of **non-local events**.

**Example — transfer** `t : m(a) -= 1; m(b) += 1` (touches `k_a` and `k_b`, non-adjacent levels):

```
φ_disp(k_a?, h_t, present)     = h_read(value-a)      descend to read the value of a
φ_disp(val(a), h_odo(v), v)     = v ≥ 1 ? h_transfert(v−1) : stop
     — the rest of the descent toward k_b? happens in mode h_transfert:
φ_disp(k_b?, h_transfert(w), ·) = h_pose(w+1)         writes b
     — and the level memory records, at the emitting k_a? node:
M[id(n), k_a?] ← "decrement already produced by this node in the current iteration"
```

The level memory plays the role of a **token between levels**: the event is non-local, but each *segment* (read `a`, write `b`) is local; `M` and the inherited attributes sew the segments together. This is the exact formalization of "homomorphisms lack inter-level storage" — and the **constructive proof that the reification of intermediate variables (Zhao/Ciardo) is replaceable**.

> **T2 (termination, bounded instance).** On a bounded schema, the set of functions `Δ → Y` is finite, hence the number of canonical forms is finite; `saturate` builds an increasing chain for the inclusion order of denotations ⟹ termination, and the number of iterations is at most the number of intermediate canonical forms (in practice: much lower, cf. Ciardo's saturation).
>
> **T3 (expressiveness).** Every non-local transition of width `w` (touching `w` levels) compiles into a level-memory transduction with a polynomial overhead in `w`. *To be proven* — this is milestone 2, the statement is now precise.

---

## 8. Second-order attributes: the main design decision

The formalization of `lookup` *forced* a decision that the initial proposal did not explicitly foresee: `lookup`'s synthesized attribute is a **DD fragment**, not a scalar value.

> **Decision D1.** The algebra of synthesized attributes is *closed under fragment construction*: `S` may contain `DD⟨Δ', Y'⟩` (second-order attributes). The aggregation `φ_aggr` becomes a canonical union of fragments; the graft `φ_out` is its dual coding.

Consequences (to be assumed, not hidden):

- **Cost**: fragments are hash-consed in the same pool as the main diagram → the memory cost of attributes is bounded by that of existing nodes + axis fragments; fragment computed tables are versioned like L1.
- **Gain**: `fold` now differs from `lookup` only in the semiring of the synthesized attribute (`(ℕ,+,×)` vs fragments) — the operation table of § 1.4 of the proposal reduces to *one* `T-NODE` rule and three algebra choices. This is the unification of operations, achieved.

---

## 9. Table of statements

| # | Statement | Status |
|---|---|---|
| Spec-τ | Each transduction carries its semantic function + induction | Proof format, to be filled per operation |
| L1 | Memoization: purity in `(id, h, version M)` | Proven (structural) |
| L2 | Commutation with quotients (admissibility A1–A3) | Proven (structural) |
| A4 | Cardinals in the aggregation (count on collapse) | Proven on the example; to be integrated into (A1)–(A3) as a condition |
| **A5** | Two algebras: the construction merge (hash-consing **is** the Bool choice) and the query semiring (parameter of the transduction) are two declarations; fold exact iff its reading factors through what the construction preserved | Statement to be proven (format (Spec-τ)); opens the ADD/weighted instance |
| **L3** | Memo (operation, node) sound ⟺ homomorphism in that argument; incremental fold ⟺ ⊕ idempotent (∃-summaries yes, cardinalities no) | Structural lemma to be proven (extension of L1/L2) |
| **A6** | Reduction↔explanation duality: the cost of a fold = the cost of re-expliciting what the reductions absorbed | Falsifiable prediction; philosophers instance (matrix, O(log n)), left-handed counter-test (constant ratio) |
| T-EXPAND / explanation principle | Operation non-constant on an axis ⇒ explanation of the axis, possible iff bounded | Proven on `lookup`; to be generalized |
| **T5** | Schema-transduction correspondence: bounded ⟺ complete transductions + canonicity; open ⟹ enumerative + non-canonical | **Main target** — the canonicity classification derived from the computation of operations |
| Typed compilation | Every operation defined on Σ (order-independent) compiles into an ordered transduction for any instance (O, R, encoding); semantics is preserved by conversions | To be proven; extends T5 (the schema forces the available compilation, the type fixes the semantics) |
| T2 | Termination of `saturate` on a bounded schema | Proven (finiteness) |
| T3 | Expressiveness of non-local events, poly(\|width\|) overhead | Milestone 2 target, statement made precise |
| D-abs / D-trans | Generalized default: absorption over any sub-domain, `D^def` designated by the controller | Milestones 4–5 extension (§ 2.4); (R-collapse)/(R-default) = special cases |
| C1 | Confluence {D-abs, D-trans, R-share}, then minimality per handle (controller, anchor, form) | To be proven; extends the T5 format |
| Work sharing | O(1) cost for the core of N replicas with invariant events (+ boundary term) | Falsifiable; benchmark n philosophers, baseline hierarchical SDD |

**What the formalization produced as new by itself**: (1) the explanation principle and theorem T5 — canonicity is not a property of the representation but a consequence of the computation of operations; (2) the graft as a missing primitive; (3) second-order attributes (D1) and the reduction of the operation table to one rule + three algebras; (4) the two algebras of "+" (A5, L3): construction and query are distinct declarations, and idempotent hash-consing is a semiring choice, not an evidence; (5) the reduction↔explanation duality (A6): every quotient pays its explanation, and the philosophers' fold is that of D-abs. These five points *came out of the specification* itself: this is the sign that the formalism has strength.

---

## 10. Open questions and implementation decisions

1. **Expansion and physical order**: `T-EXPAND` inserts explicated levels — in which physical order? The expansion must consult `Order<T>` (the reinserted level takes its declared place). This is the exact junction point between this computation and the representation plane.
2. **Memory versioning**: L1 requires `version(M)`; granularity (per node? per level? per saturation iteration?) — an implementation decision to be measured (it determines the hit rate of the computed table).
3. **Graft on defaults**: when the fragment carries `None` (absent key), the graft is constant over the continuations → the absent edge remains collapsable (the `Option` of `lookup` is *the default algebra of the result*). To be integrated into admissibility.
4. **`Set` vs `Map`**: the presence + value decomposition covers `Map`; `Set(T)` = `Map(K, Unit)` — verify that the rules cover both without duplication (expected: yes, the `Set` instance uses only presence levels).
5. **`Seq`/`Tag` generalization** (milestone 2): `Tag` = union (a level with one useful edge, already covered); `Seq` = the *list* constructor (repetition + end marker): two compilations, by positions or by enumeration of elements — this is where T5 will have to harden (the "enumeration" schema is bounded by length, not by the universe). At stake: the correctness of compiling typed operations (order-independent) into ordered transductions, for any instance.
6. **Variable encoding (absolute/relative)**: precedence per controller; the absolute node resynchronizes the accumulation. Hash-consing must normalize the encoding: canonical serialization with an encoding bit, structural tie-break by fingerprint (never by creation id). Bench invariants: absolute reading = relative accumulation; abs ↔ rel conversions involutive modulo reduction.
7. **Default designation by the controller**: `D^def` is not a property of the level; two controllers coexist at the same level with two different reductions. Conversions between kingdoms = transductions with predictable cost (the typed contract extends from orders to encodings); the oracle decides in which kingdom to compute. The computed-cache keys carry the controller's fingerprint; the level → forms index lives in the shadow plane (saturation iterates over uses, not over forms).
8. **Non-idempotent construction merge**: (A5) opens the weighted reading (weight-carrying edges, pointwise ⊕ in the ADD style), but it requires a single table indexed by weights and loses the collapsing of duplicates: to be instantiated at the ADD/MTBDD milestone, with the compatibility differential as a guardrail (a weighted fold is exact only on a lifted construction; on a Bool construction, only the cardinal is exact).
9. **Continuous δ: regions everywhere, integral at the fold**: the continuous installs itself through the key algebra (a key = a finite description of the language Σ, a region or a degenerate singleton, allowed as soon as equality of its points is decidable; the domain is continuous, the key space never is, at most countable, and the prohibition bears on one key per point of the domain, not on the isolated point). Two points resist. (i) C1 becomes relative to the region algebra: convex DBMs (canonical closure) yes, non-convex unions without a known normal form, semi-algebraic decidable (Tarski) but doubly exponential. (ii) The fold on a continuous dimension is an **integral**, not a sum: the semiring carries measures (densities), and the exact fold (A5) requires symbolic integration over the fragment; candidate: polyhedral regions + polynomial densities; point keys only count there under an atom semiring (Dirac, counting), otherwise null measure. Test instance: DDD-difference; the order O is understood on the lattice of regions, and the universe sweep only compiles if the universe is bounded to enumerable regions, otherwise an assumed free DDD kingdom.

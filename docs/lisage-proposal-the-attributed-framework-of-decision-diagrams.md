# Proposal: Lisage — the attributed framework of decision diagrams

*Synthesis drawing on the state of the art (dedicated canvas) and on the project's exploratory design notes, analyzed in the two "Critical analysis" canvases. Assumed level of ambition: a unifying framework, not yet another library.*

---

## 0. The point in one sentence

> **A decision diagram is the implementation of a lazy enumeration of a set of structured values; operations are attributed transductions over this enumeration; the order belongs neither to the data nor to the implementation, but is the *contract* that binds the two — and the compiler is where this contract is negotiated.**

Three consequences define the framework:

1. **Attribution**: the missing piece of DDD homomorphisms is a bidirectional information channel (inherited/synthesized) + a level memory. It is the operational foundation, not an extension.
2. **Typing**: a DD is typed `DD⟨X, Y⟩` (decision domain, codomain); set, relation, function and partition are instances of the same object.
3. **Order is a compiled resource**: explicit relative canonicity; reordering demoted to the rank of a conversion with predictable cost; the representation plan is reasoned about over chains of operations, not over isolated diagrams.

The formula that positions Lisage relative to the generalization/unification program:

> **Integrated Lisage = polyDD controller (structures, parameterizable) × Attribution (operations, unified) × Typed order (contract).**
>
> The framework fully unifies **operations** (it is the culmination of Linard's thesis (2009) and of *Specification of Decision Diagram Operations* (Linard, Hamez et al., 2010)); on the **structures** side, it remains general exactly insofar as level 2 *is* the parameterizable controller. The `seq/set/map/tag` algebra is only its **reference instance** — the complete validation ground, never a closed framework (demonstration § 1.2).

---

## 1. The central object: the attributed diagram

### 1.1 The type functor (reference instance)

Starting from the polynomial type algebra:

```
T ::= Atom(δ)            // δ = atom schema (bounded / pointed / open / structured)
    | Seq(T) | Set(T) | Map(T,T) | Tag(l → T)
    | T → T              // arrow resolved by level 1
```

`Seq/Set/Map/Tag` are the standard *functors*; it is the structural signature of Geneva (ΣDD, MSDD) assumed as the base algebra — everything else (atoms, cardinality, defaults) is parameterized by a **schema**. Note: this algebra is **the reference instance of the controller**, not the framework itself — everything that follows holds for any instance of the extended controller (§ 1.2), and `seq/set/map/tag` serves as the complete validation ground.

### 1.2 The extended controller: the general object

Level 2 of Lisage **is** the polyDD controller (Linard 2009), extended with three parameters:

```
Extended controller C = ( atom schema δ       // bounded / pointed / open / structured (DBM, Presburger…)
                     , type algebra Σ       // domain constructors = the variable grammar
                     //   (record, union, list, set, map ; reference: seq/set/map/tag)
                     , decomposition + order O  // constraints, not sequences
                     , reduction rules R   // quotients
                     , default algebra       // deduced from the type, overloadable
                     , default diagram D^def // what the absent edge denotes ; designated by controller (D-abs, spec § 2.4)
                     , variable encoding   // absolute at the node / relative on the edge (shift) ; precedence by controller
                     , attribution interface // allowed channels (inherited/synthesized/memory)
                                              // + semiring PER TRANSDUCTION : it lives in the
                                              //   signature of the operation (one and the same transduction,
                                              //   of semirings) ; the controller fixes the
                                              //   compatibilities
                                              // + construction merge : the ⊕ of writes,
                                              //   declared by the instance (A5 of the spec) :
                                              //   Bool idempotent today, and hash-consing
                                              //   IS this choice ; ADD/weighted in a milestone
                     )
```

Every DD category of the state of the art becomes an instance of `C`:

| Category | Salient parameters of the instance |
|---|---|
| BDD (Bryant) | Boolean Σ, δ = false, classical reduction, inherited attribution only |
| ZDD (Minato) | same Σ, **δ = absent** — the default algebra alone suffices to distinguish the two |
| MTBDD / ADD | rich codomain `Y` (the typing `DD⟨X,Y⟩` suffices) |
| EVBDD | affine edges = valuation in the controller |
| MDD (Miner/Ciardo) | enumerated atoms, static level-by-level order, saturation = static policy |
| DDD (Couvreur) | open atoms, assumed non-canonicity, homomorphisms = inherited-only transductions; its variable repetition = the unfolding of the list/set/map constructors (below) |
| SDD/HSDD (Hamez) | sets in valuation, hierarchy = recursive decomposition, saturation = self-detected policy |
| ΣDD / MSDD (Geneva) | term algebra / multisets — direct predecessors of the reference instance |
| ESDD (Devereux/Chechik 2003) | relative variable on the edge (shift), structure sharing by inclusion, canonicity by sufficient conditions; direct precursor, pre-saturation, never adopted |
| CESRBDD/RexBDD (Ciardo/Miner) | unary edge transformers: designated reduction, complement, swap |
| Sentential SDD (Darwiche) | vtree = decomposition constraint; relative canonicity = ours |

The complete bestiary (some fifty verified families, instantiated field by field in the grid above, November 2026) is in the "State of the art" canvas, § 9: each family is placed there in the parameter space of `C`, and the lines above are its poles. Recent convergences (the ESRBDD/CESRBDD/RexBDD lineage 2017-2024, SLDD 2005, LIMDD 2022) are documented there source in hand.

**Σ is the grammar of variables (the DDD repetition, tamed).** A DD has no variable sequence: its levels are the unfolding of a Σ expression. Five closed constructors: **record** (product of named fields, field order fixed by the controller: the interleaving of the golden-thread example is a record choice), **union** (the discriminant, Tag case), **list** (repetition of a variable + end value), **set** (repetition in the canonical order of elements + end marker), **map** (repetition of keys + end marker, the value carried by the edge: polyDD edge valuation promoted to constructor, contribution no. 8). The variable repetition of DDDs, until now parked as "assumed non-canonicity", becomes a typed constructor: its canonicity is a property of the chosen compilation, not an anomaly of the schema. The end marker is a **required key** (the "1" injection of μX.(V×X)+1), never an absorbable default; it cooperates with D^def without being confused with it: the marker closes enumerations, D-abs provides the sparsity of sweeps. Immediate consequence: the same constructor has several compilations. Bounded `Set(T)` compiles either as a presence sweep (one level per element, δ = absent, no marker, sparse like ZDD) or as a DDD enumeration (members only + marker); same constructor, two compilations, the oracle decides on the data profile: the first measurable arbitration of the planner.

And the architectural consequence: **operations are defined on types, independent of order** (insert, member, union, lookup, fold: the semantics of the constructor), then **translated by the controller into ordered transductions** over its (decomposition, O, encoding). New proof obligation: compilation is total and correct for every instance, and preserves semantics across conversions. This is the semantic layer that the order contract lacked: the operation is written once, the order negotiated afterwards, the planner converts without rewriting.

**Golden-thread example**: the C_philo instance (n philosophers in a ring) is unfolded end to end, from controller to counting, in the "Example: n philosophers" canvas: the reachable space, ≈ (1+√3)^n, fits in 7 nodes independent of n, the parameter n living on the shift of the input edge (figures verified by exhaustive simulation).

**Framework completeness test**: sentential SDDs must fit as an instance without *ad hoc* extension — this is the missing branch of the state of the art (knowledge compilation), and the extended controller naturally offers it a place. Two structural bonuses of the parameterization:

1. **Attributes over a semiring**: the synthesized channel lives in a semiring **carried by the signature of the transduction** (the controller fixes the compatibilities): Boolean for `exists/forall`, `(ℕ, +, ×)` for counting, probabilistic for evaluation. Counting, weighted model counting and PSDD thus enter the same unification as verification: *one and the same transduction, different semirings*. And the twin declaration, invisible until one asks the question: the **construction merge** (the ⊕ of writes). Idempotent hash-consing **is** the Bool choice; a weighted reading requires a lifted construction (weight-carrying edges, pointwise ⊕ as in ADD), where additivity is restored by distributivity at the price of idempotence. Two algebras: one writes, the other reads, and a fold is exact only if it factors through what the construction preserved (A5, L3 of the spec).
2. **Structured atom schemas**: δ can cover *structured* unbounded atoms (DBM for time, Presburger for integers) — the temporal/numerical branch of the state of the art (CDD, difference DD, NDD) integrates as a simple choice of δ. And the continuum enters through the same door: a key is always a **finite description of the Σ language** (interval, constraint, polyhedron; the singleton {c} is its degenerate region, allowed as soon as equality of its points is decidable: rationals, algebraics). The domain may be continuous, the key space never is (at most countable: a key is a word of Σ, not a point of the domain). The prohibition does not concern the isolated point but two overflows: a key *per point* of the domain (a node's children are finite in number, a pointwise partition is never exhaustive, everything else goes to D^def) and the point with undecidable equality (arbitrary computable real: no normal form, hence no hash-consing). The allowed singleton remains degenerate: null coverage, annihilated by the fold in a measurement semiring, except in an atom semiring (Dirac, counting). Concretely, δ is a **key algebra** contract (normal form, equality, coverage for R-collapse and D^def, intersection for the product, enumeration of the regions of a bounded universe), carried by the controller, never by the pool: the "node types" (discrete, interval, constraint, continuous) are instances of δ on the algebra/ side, the low-level node remains unique and untyped.

**A non-mechanism worth noting: the controller tower.** In theory one can stack several controller levels (low-level to high-level) with inter-level transformations, defining part of the sharing at controller granularity. Verdict: interesting as an *instance pattern*, not as a mechanism; almost everything there already exists. A tree-structured order is already a parameter (the vtree of SDDs is a case); a local controller per sub-order is an instance of the controller with a local anchor (the triple (controller, anchor, form) applies as-is); inter-level transformations = transductions; level-wise sharing = hash-consing of handles (form, local anchor). What the tower buys and flat+shift cannot: *irregular nested sharing* (non-uniform repetition; the regular case is already covered by unfold, spec § 2.4.4). Canonicity is modular: C1 per level, composed if the levels interface through canonical handles (the format of normalized SDDs). The tower is the installation path for sentential SDDs (vtree = structured order, sharing per element, saturation per level = the "self-detected saturation" of the HSDD lineage), to be verified at milestone 5. The low-level pool remains unique, flat and untyped: the tower lives in the algebra layer. Decision criterion (oracle): regular repetition → flat+shift suffices; irregular nested repetition → tower.

### 1.3 The diagram as a coalgebra

A `DD⟨X, Y⟩` is a decision graph over the decomposition of `X` whose leaves lie in `Y`. The fruitful point of view: it is a **finite coalgebra** over the polynomial functor induced by the decomposition. Three immediate dividends:

- **canonicity = coalgebra minimization** (the theorem to be written: classification of constructors by canonical status — see § 2);
- reductions (collapse, default-suppress, sharing) become *quotients* of the coalgebra, unified under a single notion instead of a bestiary of rules;
- unbounded `Map`/`Set` cleanly inherit the DDD status: assumed non-canonicity, syntactic equality, homomorphisms.

### 1.4 The three information flows (the heart of the proposal)

A Lisage operation is an **attributed transduction**: at each application on a node,

```
⟨inherited_h, level_memory⟩ ↓ node n ↓ ⟨synthesized_s, effets⟩
```

- **Inherited** (downward): the parameter of the DDD homomorphism — already exists;
- **Synthesized** (upward): the missing channel — `fold`, `exists`, `count` are its pure cases; it lives in a **semiring carried by the signature of the transduction** (Boolean, `(ℕ,+,×)`, probabilistic…), the controller fixing the compatibilities — this is what brings counting and weighted evaluation into the same unification as quantification;
- **Level memory**: a store per (operation × level × node), persistent across visits — this is what makes saturation a declarative operator and what fixes the case of non-local events.

**Operation implementation table**:

| Level-1 operation | Required flows | Lisage realization |
|---|---|---|
| `map f` | inherited | downward transduction (≈ DDD homomorphism) |
| `filter p` | inherited + synthesized (the verdict goes back up) | downward transduction with upward guard |
| `reduce/fold/exists/count` | synthesized | upward accumulation, terminal = aggregate |
| `lookup k` (Map) | inherited (the key) + memory (comparisons) + synthesized (the value) | the complete textbook case — impossible in the current model |
| `saturate step` | both + level memory | fixed point per level, parameterized strategy (§ 5) |
| `groupBy/join` | synthesized + inherited | bidirectional transduction |

This is the direct answer to the 2010 diagnosis (*Specification of Decision Diagram Operations*: *homomorphisms lack inter-level storage*): they lacked two channels and a memory; here they are, and the "non-homomorphic operations" (lookup, insert, join, groupBy) become homomorphic *for the attributed semantics* (they only were for the mono-directional semantics). And the memory law that refines "homomorphic": a memo (operation, node) is sound iff the operation is homomorphic in that argument; a fold holds during saturation iff ⊕ is idempotent (the ∃-summaries yes, the cardinalities fold at the fixed point); and the construction semiring (the merge of writes: hash-consing is the Bool choice) is a declaration distinct from that of the query: a fold is exact only if it factors through what the construction preserved (A5, L3 of the spec).

---

## 2. Canonicity: a classification theorem, not a slogan

**Target statement.** For each constructor and each atom schema:

| Domain | Status | Ground |
|---|---|---|
| Bounded atoms + bounded Seq/Set/Map | **canonical** | minimization (minimal automaton/coalgebra); `Set` = characteristic function, `Map` = functional coalgebra sorted by canonical keys |
| Pointed atoms (default δ, bounded encoding) | canonical + **default algebra** | default suppression generalizes BDD (δ = false) and ZDD (δ = absent); δ is deduced from the type, overloadable per operation |
| Open domains (unbounded Int/String, infinite Map) | **assumed non-canonical** | DDD style: equality by syntactic identity, attributed operations |

Two corollaries worked out from the very start of the project (not "later"):

1. `Canon(T, D, O, R, δ)` — the canonical form is relative to the quintuple (type, decomposition, order, rules, defaults); *semantic equality between two different plans = equality of decodings*, never structural identity.
2. The **semantic fingerprint** (hash of the quintuple + hash of the reduced coalgebra) serves as a cross-representation cache key — two physical plans of the same object share their operation results.

---

## 3. Orders: typed, composable, negotiated

### 3.1 `Order<T>` as a first-class value

```
Order<T>  ::=  atom(δ, π)              // permutation by type and context
            | seq(O, tail)             // position × reste
            | set(O)                   // canonical order of elements
            | map(O_key, O_val)
            | tag(l → O)               // discriminant before payload
combinateurs : project / interleave / refine / coarsen
conversions  : DD⟨T, Y⟩ @ O₁  →convert→  DD⟨T, Y⟩ @ O₂   // first-level node, predictable cost
```

- **Type-wise permutations π** are integrated *and constrained*: π lives in `Order<T>` and **alignment becomes a typing** — two operands compose only under a common order, which the compiler must *introduce explicitly* (convert, or the module's canonical order, or specialization of the operation for the pair (O₁,O₂)).
- Dynamic reordering is not removed but **demoted**: a measured fallback of the planner, never a default mechanism. The cost of a conversion is predicted before being paid (Sølvsten & van de Pol brick: prediction of the memory needs of operations).

### 3.2 What the design notes left open and what must be decided

- The **Semantic DD is never a mandatory gateway**. It is materialized on demand: for values known to be wanted under several orders (hot values), otherwise one compiles directly from level 2 to the physical plan. Reason: the semantic form can be exponentially larger than the best physical plan.
- **The planner is two-dimensional**: static over known operation chains (which covers the `map → filter → exists` pattern), dynamic over the unknown (runtime higher-order functions). The pure language is an asset: everything is rewritable/memoizable/movable.

---

## 4. The compiler: representation planning

Pipeline (a single IR, three classes of annotations — decided as in the critical analysis):

```
Level 1: pure typed language (HOF, seq/set/map/tag)
              │  normalization + fusion (the classic)
Level 2: attributed IR — the extended controller (§ 1.2), instantiated:
           operations as typed transductions,
           types decomposed, semantic orders (constraints, not sequences)
              │
        REPRESENTATION PLAN
        (profitability oracle : concret vs DD@O₁ vs DD@O₂ …)
              │
Level 3: physical IR — same language + convert/materialize nodes
           + physical order + encoding + placement
              │
        core: Lisage pool (forms, handles); Sylvan/CUDD: witnesses
```

**The profitability oracle is research brick no. 1** — the one that all existing approaches postpone. First model, deliberately coarse but *predictive*:

```
profitable(dd, op-chain) ≈  encoding_size(domain) × expected_density
                          <  Σ concrete_cost(ops)  −  construction_cost(dd)
                          +  downstream_query_value (quantification = jackpot)
```

fed by: the profile of the data distribution, node prediction (maximal cuts), and learned feedback (the containerized MCC 2011–2018 data are exactly the dream training corpus). The oracle can be learned later; it must exist in analytical form from the prototype on.

---

## 5. Declarative saturation

```
saturate : (op : attributed transduction, D : DD⟨S, Bool⟩) → DD⟨S, Bool⟩
```

- **Instances**: static level-by-level strategy (Ciardo) and self-detected strategy (Hamez) = two *policies* of the same operator, selected by the plan.
- **Non-local events**: in Lisage they are written natively — the level memory carries the intermediate state between the levels that an event crosses; no more need to reify intermediate variables in the Zhao/Ciardo style. This is the expressiveness theorem to be proved: *every multi-level transition is a Lisage transduction with level memory, with an explosion at most polynomial in the width of the event*.
- **Memory scope**: the level memory lives in the node (co-localized attribute table), which makes saturation one of the first operations *naturally cache-aware* in the field.

---

## 6. Memory: the two-plane store (the imaginative part)

The fundamental conflict — hash-consing = addressing by **content**, locality = addressing by **usage** — is resolved by not choosing:

- **Content plane**: the node pool, hash-consed, IDs in creation order, *never moved*. This is where canonicity, identity, the fingerprint live.
- **Shadow planes**: per dominant operation phase, a compacted replica ordered by *observed access affinity* (profile of the previous phase), built during GC periods, disposable, without semantic identity. Microsoft's mark-sweep-shift is its ancestor; the difference is that here the placement is *driven by the attributed transductions themselves* (the level memory knows the access neighbors).
- Attributes (persistent synthesized values, transduction caches) are **co-localized with the nodes in both planes** — the same mechanism serves semantics (§ 1.3) and locality (§ 7 of the locality document): one stone, two birds, zero duplicated structure.

Counter-intuitive but defensible: we *accept* paying one replica per phase. A DD rarely fits in caches anyway; a well-ordered replica that saves 30% of cache misses on the dominant phase pays back its construction.

---

## 7. The language (level 1) and the user promise

Nothing new under the sun — that is intended. The user writes:

```
type State = { pc : Int, stack : Seq Int, vars : Map String Int }

step : State → Set State

let reachable = fix(step, init)                 // sat
let good = exists (s -> s.pc = 42) reachable    // synthesized
let norms = map normalize reachable             // inherited
```

- **Two doors** to the DD world, as discussed: opportunistic (the runtime converts an intermediate value) and symbolic (quantifications). In Lisage, the two doors are the *same* mechanism: the oracle decides that `reachable` deserves a DD representation; `exists` and `map` become transductions.
- Closures (HOFs) only go down into the DD if they *reify* into an attributed transduction (compilable); otherwise `materialize`. Expallowed decision of the planner, not a silent failure.

---

## 8. Implementation: the layers

```
lisage/
├── lang/         level 1: types, reference interpreter (concrete syntax: surface decision, roadmap § 10)
├── algebra/      level 2: extended controller — schemas, Order<T>,
│                 attributed transductions (semirings),
│                 reductions (quotients), fingerprints, canonicity,
│                 instance library (BDD, ZDD, MTBDD, MDD, DDD,
│                 HSDD, sentential SDDs…) and reference instance seq/set/map/tag
├── plan/         profitability oracle, representation plan, conversions
├── core/         level 3: multi-controller pool (forms, handles), IDs,
│                 GC, two-plane, placement, co-localization
├── backend/      sylvan | cudd : witnesses (classic BDD oracle, baselines),
│                 degraded mode
└── bench/        differential, property-based, MCC, non-local events
```

Layer rules (sharpness criteria, not slogans):

- `algebra/` does not know `core/`: it only sees the "store/retrieve a node by key" interface, which `core/` implements; the witnesses (`backend/`) live on the `bench/` side and never go back up;
- the `core/` ↔ `algebra/` boundary is **verifiable**, not a slogan: everything that reads `Val(ℓ)`, `δ(ℓ)`, `Y`, an anchor verdict or a cardinality lives in `algebra/` (instantiated by the controller); everything that compares keys and pointers lives in `core/`, which **never inspects ⊕**: it routes and memoizes, merges are opaque callbacks. Reductions are distributed the same way: R-share is syntactic (pool side); R-collapse, R-default and D-abs read the type (controller side): the "+" and the reductions live on the same floor;
- `core/` is the native path from milestone 1: abstract variables, relative forms, controller-wise defaults and handle-wise canonicity are storable in no existing package. Sylvan/CUDD are no longer backends but **witnesses**: differential oracle when a controller is configured as a classical BDD (identical reproduction of CUDD), bench baselines; the *degraded* mode (transductions linearized into binary operations, without level memory or two-plane store) survives as a comparison route. The Lisage degree remains progressive: BDD → DDD → Lisage;
- `bench/` is a first-class citizen from the first commit: differential testing (interpreter vs DD), property-based (idempotence `reduce∘reduce = reduce`, commutation of quotients), and the falsifiable A/B/C triptych (naive / fixed-order DD / Lisage+planner).

Sequencing (assumed shortcut of the design notes, amended):

1. **Milestone 1** (8-10 weeks): minimal `core/`: arena, `NodeId`, keyed hash-consing (controller, anchor, form), reductions spec § 2.3; `Atom bounded + Set + Map`; inherited+synthesized transductions; `map/filter/exists/fold`; differential tests against the reference interpreter, Sylvan/CUDD as witnesses on the classical BDD instance. → *already answers "does the attribute replace today's contortions?"*
2. **Milestone 2**: `Seq/Tag`; level memory; `saturate` with both policies; non-local events. → *the expressiveness theorem.*
3. **Milestone 3**: complete typed Order, conversions with predictable cost, analytical profitability oracle. → *the representation plan.*
4. **Milestone 4**: two-plane pool, attribute co-localization, cache metrics; activation of relative forms (shift) and of the generalized default (D-abs), provided by the core schema from milestone 1, measurement on n philosophers against hierarchical SDD encoding. → *locality and sharing.*
5. **Milestone 5**: complete language, HOF, MCC corpus, learned oracle; installation of sentential SDDs (completeness test, contribution no. 6; the controller tower as instance pattern, § 1.2).

---

## 9. Expected contributions (and therefore falsifiable)

| # | Contribution | Falsified if… |
|---|---|---|
| 1 | Attributed semantics unifying homomorphisms, folds and saturation | a non-local event forces an unavoidable reification |
| 2 | Canonicity classification theorem by constructor/schema | a bounded case requires a non-canonical representation (or vice versa) |
| 3 | Order = typed contract; conversions with predictable cost | the planner never beats a "well-chosen fixed order" on MCC |
| 4 | Profitability oracle | it predicts no better than a heuristic threshold |
| 5 | Two-plane pool | shadow planes cost more than they return on real workloads |
| 6 | Extended controller: every classical DD category expresses as an instance (completeness tested on Darwiche's sentential SDDs) | a category of the landscape requires an *ad hoc* extension of the framework |
| 7 | Attribution over semirings: counting, weighted model counting and probabilistic evaluation = one and the same transduction | a useful semiring requires a distinct operation semantics |
| 8 | Unary edge transformers (shift, default, complement, swap): a single structural valuation, an instance of the controller; storage and work sharing by translation invariance (spec § 2.4) | on N replicas with invariant events, the saturation cost grows in O(N) instead of O(1) + edge; or the sharing does not show up on n philosophers |

A single one of these lines that holds already makes a paper; 1, 3 and 6 are those that directly extend Linard's thesis (2009) and *Specification of Decision Diagram Operations* (2010) — 6 is its completeness criterion. 8 extends the edge valuation of polyDD from the algebraic payload to the structure.

---

## 10. What the proposal does not retain from the design notes

To mark the choices:

- **No** Semantic DD as a systematic pivot (explosion risk);
- **No** code generation at user-program compile time before milestone 5 (too many layers to debug simultaneously);
- **No** fully automatic planner before having an honest analytical oracle;
- **Sharp reversal**: a *specific* low-level DD library from milestone 1, not at milestone 4. "Sylvan first" assumed classical nodes; relative forms, the controller-designated default and handle-wise canonicity made our nodes incompatible with every existing package, and starting from an existing engine would empty the design of its substance. What remains **no**: a general-purpose DD library for the world; `core/` is the storage layer of our forms, nothing more (no dynamic reordering in particular: order is a contract of the controller, conversions handle it). Sylvan/CUDD become **witnesses**: differential oracles on the classical BDD instance and bench baselines. Assumed risk: storage on the critical path from the first commit, covered by the reference interpreter (truth) and property-based testing (C1, quotients) from the first commit.

---

## 11. The name

"**Lisage**" — a single name for the theory, the framework and the tool. Lisage is the art of preparing a Jacquard fabric: the grid is read on two axes (the warp goes down, the weft goes up — exactly the inherited and synthesized attributes), and the reader encodes the pattern on punched cards, the direct ancestor of the cards that founded computing (Jacquard 1804 → Babbage → Hollerith). Etymology: *lire* — the oracle reads the structure, the code is a readable glossary. The name thus literally says "encoding decisions into a machine-executable form". Assumed retroacronym: **L**anguage, **I**nstances, **S**aturation, **A**ttributes, **G**raft, **E**xecution — the six pillars of the framework. Collisions checked and nil: crate `lisage` free on crates.io, no homonymous software on GitHub (only the textile industry), unambiguous web query. The previous "ADA" is abandoned: major collision (the Ada language, the company AdaCore, the crates `ada` and `ada-url` taken, Cardano's ADA token drowning out the engines, and AADD already taken in the DD literature by Affine Algebraic Decision Diagrams).

# Critical analysis of the design note "Decision Diagrams, IR and memory locality"

*Method note: the analyzed document is an exploratory project design note (2026); its bibliography was flagged there as unverified. Verifiable references were therefore verified, and the analysis separates what belongs to the document from what belongs to the substance.*

---

## 1. What holds — and holds well

1. **The representation locality / execution locality distinction** is correct and rarely made expallowed. Most placement work concerns itself with the former and observes the latter empirically.
2. **The separation into four dimensions** (semantic order, physical variable order, memory layout, execution order) is a good grammar of analysis. Important caveat, however: see § 4.7 — it is a good *specification discipline*, but not a factorization theorem.
3. **The methodological caution** (existing backend, reference interpreter, differential tests, property-based testing, fuzzing) is the right strategy for a prototype. Nothing to object to on the substance.
4. **The final conclusion** — "it is the objects used together by the dominant operations that should guide placement" — is the right question, correctly formulated.

The core of the problem is not in these points. It lies in three blind spots: a research problem deferred without being named, a "semantic" canonicity poorly posed, and a factorization between order and operations that is too clean — precisely what *Specification of Decision Diagram Operations* (Linard, Hamez et al., 2010) shows to be untenable.

---

## 2. The language project: the cost model is the subject, it is relegated to an apin-flightix

### 2.1 "Transparent exploitation": a hard research problem, not a feature

Lane 1 (`map`/`filter` "detects" that a value lends itself to a DD representation) presupposes a **profitability model**: knowing *a priori* when the symbolic construction of a value + the specialized operation cost less than concrete execution. This is exactly the open problem of Linard's thesis (2009) and of polyDD (Linard et al., 2010) (when is a category of DD "dedicated to a given problem" profitable?), and the note treats it as a future implementation detail. It should be the project's research objective no. 1, with a first cost model even if coarse: domain size, expected density, encoding cost, number of subsequent symbolic queries (a symbolic representation is never profitable for *one* operation, only for a *sequence*).

### 2.2 The major blind spot: composing operands of different orders

The document says (§ 3.2) that "the encoding may dein-flight on the type and the context", but never confronts the consequence: if two symbolic values built in different contexts (hence different orders) are combined, inter-order `apply` is catastrophic — the operation on two DDs with incompatible orders can blow up exponentially in intermediate size, or require a costly permutation first. A function compiled to DD is a *closed* object whose variable order is part of the calling contract. The "physical plan" must therefore include an **order alignment policy** as a first-class primitive (reorder the smaller operand? maintain a canonical global order for the module? pay the permute?), otherwise the language composes incompatible representations. This is the only real conceptual problem the note does not see, and it is linked to the order/operations entanglement (§ 4).

### 2.3 Lane 2 is sound

The "expallowedly symbolic" lane (`exists`, `forall`, queries) is on the contrary well posed: quantification is a natural *semantic abstraction*, and the compiler freely chooses the encoding underneath. This is the level at which the two lanes should in fact meet: the compiler can reuse the symbolic lane as a specialization target for the transparent lane, the cost model deciding on the conversion.

---

## 3. Semantic IR / Physical IR: the "mathematical canonicity" is poorly posed

### 3.1 There is no encoding-indein-flightent canonical form

§ 2.1 presents the Semantic DD as "mathematical canonical form, even if not materialized". Yet in the entire literature (Bryant, Darwiche, the polyDD controllers), canonicity is *always relative to a triple*: (variable order, reduction rules, default convention). A "conceptual" semantic order is itself a choice — simply a different choice. The semantic level cannot be canonical; it can only be **defined up to equivalence** (the class of all physical representations of the same function). This is in fact a more useful definition: the Semantic DD is an equivalence class, and the physical plan is a choice function within this class. Formulated this way, the Semantic/Physical distinction becomes clean.

### 3.2 "Annotations rather than a separate language": the documented risk

The idea of a single IR with annotations is defensible (it is roughly the LLVM strategy), but the experience of real compilers shows the failure mode: annotations mutate into first-class nodes as soon as a pass must reason about them. The correct dividing line is:

- **conversions** between representations are *semantic* operations (they change the object, not just its cost) → first-class nodes of the IR;
- **hints** (traversal plan, placement) are semantically *unobservable* metadata → annotations.

The note brushes against this distinction ("expallowed nodes indicating conversions or costs") but treats it as a detail. This is the most important architectural point of the document and it should be settled before any prototype.

### 3.3 The list of reductions mixes three different natures

§ 3.1 groups together mechanisms that are not of the same nature:

1. **Canonical reductions preserving the function**: sharing of identical sub-DDs, elimination of redundant tests. These are the only ones that deserve the name *reduction*: they change neither the represented function nor the interpretation of the diagrams.
2. **Changes of canonical convention**: zero-suppressed is *not* a reduction of the BDD — it is another algebra. The missing branch no longer has the same meaning (never selected vs. by default), so changing convention changes the interpretation of *all* diagrams in the system, not just of one subgraph. Same for collapse/don't-care, which assumes three-valued logic or partially defined functions. These choices belong to the controller (in the polyDD sense), not to the "reduction" level.
3. **Non-observable physical compression**: packing, field compaction.

Moreover, the document ranks **hash-consing** in category 3, whereas it is the very mechanism of canonicity: the unique table makes node identity a consequence of content. Hash-consing is not a memory optimization among others; it is what makes structural equality decidable in O(1) and it is precisely what **constrains** placement strategies (see § 4.5).

### 3.4 "Build directly in the physical order": the static plan cannot exist

§ 3 claims that destructive reordering can be avoided by building directly according to the physical plan. The elided difficulty: the good order depends on the *intermediate results* of the construction, not only on the initial semantic description. This is the purpose of dynamically triggered sifting in CUDD (node thresholds). The "physical plan" must therefore be a *reactive process* (observable: counters, thresholds, replanning), not an initial decision. The note presupposes it static: direct contradiction with the order/operations entanglement (Linard 2009; *Specification of Decision Diagram Operations*, 2010).

---

## 4. Memory locality: globally correct answer, three notable gaps

### 4.1 Omission of multi-core / NUMA

Sylvan appears in the references, but the whole note is single-threaded. Yet the dominant locality of a multi-core engine is **NUMA affinity and contention on the tables** (unique table and computed table shared), not the layout of the nodes. Existing entry point exactly at the junction of the note's themes: *Multi-core On-The-Fly Saturation* (van Dijk, Meijer & van de Pol, TACAS 2019, [DOI](https://doi.org/10.1007/978-3-030-17465-1_4)) — saturation, multi-core and variable order interact head-on there.

### 4.2 `apply` is a traversal of the *product* of two DAGs

The most fundamental limit of per-diagram placement, not stated in the document: a binary operation visits *pairs* of nodes, one in each operand. The locality of `apply` therefore depends simultaneously on both layouts, which interfere — no layout of a single diagram can capture the access pattern of the product. The two correct conclusions the document does not draw: (i) placement must be reasoned *per operation* (even per pair of frequent operations), not per diagram; (ii) for dominant binary operations, the real lever is the *joint* layout (co-placing two diagrams often combined), which joins the mark-sweep-shift of the Microsoft library but generalizes it.

### 4.3 Tables are often dominant

The note optimizes nodes; yet in a real package, the computed table and the unique table frequently represent the bulk of memory traffic and cache misses (the unique table is systematically traversed at each node creation; the computed table at each subproblem). A benchmark that measures only cache misses *on nodes* can optimize the minor component. (Support: CacBDD — Lv, Su & Xu 2013, "A BDD Package with Dynamic Cache Management", [DOI](https://doi.org/10.1007/978-3-642-39799-8_15) — shows that cache management alone is a subject.)

### 4.4 The fundamental tension not identified: content addressing vs. usage addressing

Hash-consing assigns nodes an identity *based on content*; access-affinity placement wants to arrange them according to *usage*. These two principles produce contradictory assignments. Interesting practical consequence that the document could exploit: in standard hash-consing, IDs are assigned **in creation order**, so the default layout captures the temporal affinity of *construction*, not that of subsequent operations. Hence two strategies consistent with this observation: (a) reassign IDs during compaction according to the profile of the *next phase* (generalization of mark-sweep-shift); (b) give up fine-grained placement and optimize the tables — which brings us back to § 4.3. The document opposes compaction/profiling without seeing that the underlying question is this addressing conflict.

### 4.5 Bibliography: partially unverifiable, two errors identified

The note acknowledges this itself, but here is the state of the verifications:

| Cited reference | Status |
|---|---|
| *Binary Decision Diagrams on Modern Hardware*, FMCAD 2023 | Confirmed: Samuel Pastva (Masaryk Universite), the `is.muni.cz` PDF is consistent |
| *TeDDy* → link `github.com/ahamez/eddy` | **Error**: this link points to EDDy (Alexandre Hamez's library). TeDDy is another project: Mrena, Kvassay & Zaitseva, "TeDDy: Templated decision diagram library", SoftwareX 2024 ([DOI](https://doi.org/10.1016/j.softx.2024.101715)). Both deserve to be cited, separately |
| *Design of a Pointerless BDD Package* (2001) | Not found in bibliographic databases under this exact title — to be re-verified before use |
| Yang & O'Hallaron 1997 | Plausible DOI but not confirmed during the verifications |
| Chilimbi & Larus 1998, Sylvan, CUDD, Microsoft DecisionDiagrams | Solid |

And two recent absentees directly in the project's target: BDD manipulation **in external memory** (Solvsten, van de Pol et al., arXiv 2021, [DOI](https://doi.org/10.48550/arxiv.2104.12101)) and **prediction of the memory demands of BDD operations** (Solvsten & van de Pol, TACAS 2023) — the latter is exactly a piece of the cost model missing from § 2.1.

### 4.6 Benchmark: methodology to harden

The benchmark plan (§ 7) is good, but two pitfalls are not flagged:

1. **Density/hashing confusion**: compaction changes the assignment of IDs, hence the probing sequences of the hash tables. Measuring "compaction on/off" simultaneously captures the density effect and the hashing distribution effect. One must control (randomize the IDs reproducibly, or measure with/without re-hash).
2. **Gaps in the measurements**: hit-rate of the computed table, relative size of the tables vs. number of nodes, controlled construction order (same function, different construction orders → different layouts). And realistic workloads already exist: the models of the Model Checking Contest serve as a proven test bench for this whole lineage of tools.

---

## 5. The point missing from the whole note: the order/operations entanglement

The document's final remark is not developed there, yet it is its critical keystone: **order and operations are entangled, and homomorphisms lack an inter-level information channel**.

### 5.1 Formulation of the diagnosis

In the DDD model, a homomorphism is a descending operator that decomposes level by level: at each variable, it chooses the successors. The only information channel is the operator's *parameter* (Thierry-Mieg's combinators: `Collected`/`IfThenElse`/etc.), so all contextual information must be frozen into the operator *before* the descent. There is:

- **no upward channel** (synthesized attributes: information computed in a subtree is not accessible to the parent level during the operation);
- **no level memory** (no state stored between two visits to the same level, apart from the diagram itself);
- and therefore no choice but to *reify the state into the variables* (intermediate variables) when a computation depends on the conjunction of several levels — with the cost that this implies in diagram size.

### 5.2 The literature empirically validates this diagnosis

This is exactly the problem encountered by **saturation with non-local events**: transitions spanning several levels break the validity/efficiency of level-by-level iteration. The Ciardo/Zhao lineage works around it through *event localization* (rewriting multi-level events as local events via intermediate levels) — that is, precisely the state reification denounced above, done by hand. The two diagnoses (order/operations entanglement, absence of inter-level storage) are two faces of the same phenomenon: **the level is not a watertight encapsulation boundary**, and current formalisms pretend it is.

### 5.3 Two consequences for the project

**Toward semantics.** The natural formalization exists: current homomorphisms correspond to *inherited attributes* (information only descends through parameterization). What is missing are *synthesized attributes* and a level memory — in other words, moving from descending transducers to **attributed grammars** on the diagram (or to stateful tree rewriting). A generalized polyDD "controller" should specify, per DD category, the two flows (inherited/synthesized) and the inter-level storage policy. This is, to date, a real gap in the literature, and the natural extension of *Specification of Decision Diagram Operations* (2010).

**Toward memory.** There is a happy convergence with the locality topic: if inter-level information must be stored somewhere, the question of *its* placement arises at the same time as that of the nodes. Storing the attribute *in* the node (or in a co-located parallel array) rather than in a separate structure solves the semantic problem and the locality problem together: the note's `TraversalPlan`/`PlacementPlan` would gain a fourth concept, an `AttributeStore`, whose layout is an integral part of the physical plan. The benchmark of § 7 of the document should therefore add a variant: operations with inter-level attributes, with and without co-location of the attributes.

---

## 6. Synthesis — five priority corrections

1. **Promote the cost model** (when is a symbolic representation profitable?) from future implementation to central research objective; rely on memory-demand prediction (Solvsten & van de Pol 2023) as a first building block.
2. **Add the order alignment policy** to the primitives of the physical plan; it is the untreated consequence of "the encoding depends on the context".
3. **Settle the IR**: conversions = first-class semantic nodes; hints = unobservable annotations. No intermediate status.
4. **Reclassify the mechanisms**: canonical reduction / change of canonical convention / physical compression — three watertight categories, and hash-consing on the side of the foundations of canonicity, not on the side of optimization.
5. **Merge the two threads** (order/operations entanglement + inter-level channel): saturation with non-local events is the textbook case; a prototype of attribute homomorphisms on this case would validate this thesis and give the project its distinctive research result.

---

## 7. References added during verification

- Pastva, S. — *Binary Decision Diagrams on Modern Hardware*, FMCAD 2023 (confirmed).
- Mrena, M., Kvassay, M., Zaitseva, E. — *TeDDy: Templated decision diagram library*, SoftwareX 2024. https://doi.org/10.1016/j.softx.2024.101715
- van Dijk, T., Meijer, J., van de Pol, J. — *Multi-core On-The-Fly Saturation*, TACAS 2019. https://doi.org/10.1007/978-3-030-17465-1_4
- Solvsten, S. C., van de Pol, J., et al. — *Efficient Binary Decision Diagram Manipulation in External Memory*, arXiv 2021. https://doi.org/10.48550/arxiv.2104.12101
- Solvsten, S. C., van de Pol, J. — *Predicting Memory Demands of BDD Operations Using Maximum Graph Cuts*, TACAS 2023.
- Lv, G., Su, K., Xu, Y. — *CacBDD: A BDD Package with Dynamic Cache Management*, 2013. https://doi.org/10.1007/978-3-642-39799-8_15
- Minato, S. — *Streaming BDD manipulation*, IEEE Trans. Computers 2002. https://doi.org/10.1109/tc-2002.1004587

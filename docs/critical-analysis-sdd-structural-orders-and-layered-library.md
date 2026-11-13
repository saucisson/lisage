# Critical analysis — design note « Structural ΣDDs: seq/set/map/tagged union »

*Critical analysis of the supplied design note (2026), succeeding the note « Decision Diagrams, IR and Memory Locality ».*

---

## 1. What this note actually delivers

Three ideas in it are solid and worth keeping:

1. **The trio of interpretations** (structured value / function over dimensions / set of paths) and the operational consequence: `map`-on-each-path ≠ `reduce`/`exists`-over-the-set-of-paths ≠ closure (`saturation`). This three-family taxonomy (path transformation, aggregation across paths, closure) is the right reading grid for all level-2 operations.
2. **Relative canonicalization**: `Canon(T, O_s, R)` — canonical for the triple (decomposition, order, reduction rules), not absolute. This is exactly the correction to bring to the « canonical mathematical Semantic DD » of the previous note, and the note integrates it well here.
3. **Order as an algorithmic resource** and the program « compile the expression *under* the order that downstream operations will need », rather than build then reorder. This is the thesis of the order/operations entanglement (Linard 2009; *Specification of Decision Diagram Operations*, 2010), reformulated as a compilation problem: the right reformulation.

The rest calls for serious reservations, in particular about the claimed novelty and about a canonicality blind spot. Details below.

---

## 2. The starting point: a lineage internal to the project's ecosystem

Verified: **ΣDDs** are by **Didier Buchs and Steve Hostettler** (2009), the Geneva group, with which the polyDD authors have co-published (AlPiNA, CosyVerif). The same group published the **Multi-Set Decision Diagrams** (Levi & Hostettler, 2009). In other words, the note proposes extending a lineage *internal to the project's ecosystem* (Geneva, LIP6): this is not an architecture set on a foreign literature, it is an internal continuation, and it must be evaluated as such (in particular: why have ΣDDs not already been extended this way? The 2009–2012 answer was polyDD, Linard et al., ACSD 2010).

---

## 3. The « Structural DD »: less new than it seems, and that is good news

The proposal of a DD over `Seq/Set/Map/Tag` is presented as a conceptual leap. In reality, each layer has a direct, known antecedent:

| Proposed layer | Known antecedent | What is at stake there |
|---|---|---|
| `Seq` (suffix sharing, sequence language) | DD over sequences = **shared trie = minimal automaton** (BDD/ZDD seen as automata); **Queue BDDs** by Godefroid & Long (LICS 1996) | A DD over sequences at variable positions *is* a minimal deterministic automaton; suffix sharing is determinization/minimization |
| `Set` (canonicalization + presence DD) | **Multi-Set Decision Diagrams** (Levi & Hostettler 2009); canonical form = characteristic-function DD over element variables | Canonical iff total order on element encodings; nothing more exotic than a characteristic-function BDD |
| `Map` (key→value relation) | HSDD (edges valued by sets) + functional MTBDDs; `Map<K,V> ≈ DD<K, Option<V>>` (the note says so itself) | The « canonicalization constraint on keys » = key order = one more variable ordering |
| `Tag` | Term constructors of the ΣDDs themselves; typed unions of MDDs | The layer closest to the original ΣDDs |

**Consequence for the project**: the « Structural ΣDD » is not a new primitive, it is a *synthesis* — and that is precisely the polyDD program (a framework generalizing DD categories) instantiated on the Geneva structural signature. The note itself says, in conclusion, that one will have to « see whether it falls back onto known structures »: yes, largely, and the only place where it does *not* fall back onto the known is the place the note does not explore: **operations and their compilation** (§ 7). The correct slogan would be: the constructors are known, order-as-compilation is the real subject.

---

## 4. The canonicality blind spot: the requested proof is false in general

The note proposes to prove: « for any finite structure, its reduced representation is unique ». This is **false without strong hypotheses**, and the history of the field shows it:

- For unbounded `Seq`, the canonicality fixed point is **automaton minimality** — attainable only for regular representations. The **Queue BDDs** of Godefroid & Long are the proof by failure: for unbounded queues, the canonical representation does not exist, and approximation was needed (loss of canonicality, of O(1)-equality, of union in reasonable time).
- For `Set<T>`/`Map<K,V>` over **infinite** atoms (the target language has unbounded `Int` and `String`!), the canonical form of a finite set of values exists (characteristic function over the encoding), but its *support* (the dimensions) depends on the encoding of atoms — and changes with each insertion outside the domain already seen. Canonicality is therefore stable only **relative to a fixed atom-encoding schema** — one more parameter in `Canon(T, O_s, R)`, which the architecture must make explicit (atom schema = which dimensions exist, bounded or pointer encoding).
- The unbounded-in-keys `Map` case is the worst: it is exactly the territory of Couvreur's DDDs, whose **non-canonicality is an assumed choice** for this reason. The note should say: bounded `Seq`/`Set` → canonical; open domains → DDD-style non-canonical + homomorphisms, with equality by syntactic encoding, not by function.

**Correction to bring to the program**: replace « the reduced form is unique » with « the reduced form is unique *relative to* (atom schema, decomposition, semantic order, rules) » and classify constructors by canonical status. This is a classification theorem to write — it is there, and not in the constructors, that the theoretical contribution lies.

---

## 5. Non-Boolean terminals: the answer is correct, and it is an already-known result

The question of non-Boolean terminals in the hierarchy receives in the note a correct answer (each sub-DD is a function; the parent node *composes* these sub-functions; the sharing `D₁ = D₂` follows from it). But one must name what the answer rediscovers: it is **the very semantics of SDDs/HSDDs** — an edge of an HSDD carries a set represented by a sub-DD, and O(1) equality by node identity is precisely Hamez's hierarchical sharing. Likewise, the « DD = set (interpretation A) vs DD = function (interpretation B) » duality is the classic **language/function** duality (BDD vs MTBDD), and the fix of « a single global function » — *the DD represents a set of paths, a union* — is exactly the semantics of DDDs (a DDD denotes a set of assignments, union by construction).

However, two elements of the note are genuinely good and worth keeping:

1. **The default algebra by type** (`Bool → false`, `Set → ∅`, `Map → ∅`, `Option → None`, `Int → 0`): this is the correct generalization of BDD conventions (default = false) and ZDD conventions (default = absent), derivable from the type and overloadable by the operation. To be integrated into the polyDD controller as a full-fledged parameter — it is concrete and new in this form.
2. **The functional terminal** (`DD : Bool → (Int→Int)`, terminals = closures) and the immediate caution about extensional equality: correct. Sharing will happen on the canonical representation of the *syntax* (canonical IR of the closure), never on the semantics. It is a structural constraint of the model, well seen.

---

## 6. Semantic order / physical order: convergence, but three hidden costs

The architecture `canonical SemanticDD → (permutation π per type, physical order) → PhysicalDD`, with recompilation rather than destructive reordering, converges with the previous analysis (note « DD, IR and Memory Locality »), whose canonicality point it in fact corrects. Three costs remain hidden:

1. **The Semantic DD itself can be exponentially larger than the best Physical DD.** « Order-independent » canonicality remains relative to a *semantic decomposition* — which is a disguised order choice. For some functions, no single semantic decomposition is compact while several physical orders are. Practical consequence: the « recompile from the Semantic DD » strategy amounts to full reconstruction — on very large state spaces, this is precisely the cost one wanted to avoid. The Semantic DD must therefore be *optional*: materialized for objects known to be reused under several orders, never as a systematic gateway.
2. **The planner must know the future.** « Compile `x` directly under the order that `map f` then `exists p` will need » assumes an operation chain known at compile time. With higher-order functions passed as arguments, the downstream is a *runtime* value: the static planner reduces to heuristics, and dynamic reordering must still be in the toolbox — the CUDD problem does not disappear, it is demoted to fallback status.
3. **Composition is deferred, not solved.** The note itself identifies « the composition of two Semantic DDs with different decompositions » as the next difficulty. It is *the* problem: with permutations π **per type and per context**, the incompatibility space between operands literally explodes (π₁ ≠ π₂ on a single type suffices to break compiled binary operations). The architecture needs an explicit alignment policy — this was already the finding of the previous analysis; per-type πs make it worse.

---

## 7. Operations: the note stops exactly where the Lisage proposal begins

The most important point, raised as an objection: « we also need **non-homomorphic operations** ». The note admits it (§ 8: `lookup`, `insert`, `join`, `groupBy` are not homomorphisms; § 13: three families), but proposes **no mechanism** — it refers to a vague « interpreter over the order ».

Yet the three-family taxonomy it gives is exactly the grid that the idea of attributed homomorphisms in the Lisage proposal lacks, and it superimposes on it term by term:

| Family (note) | Direction of information | Mechanism |
|---|---|---|
| Path transformation (`map`) | **inherited attributes** (downward) | classic DDD homomorphism |
| Aggregation across paths (`reduce`, `fold`, `exists`) | **synthesized attributes** (upward) | channel absent from current homomorphisms — this is the missing mechanism |
| Closure (`saturation`) | both + **level memory** (fixed point per level) | Hamez saturation = instance; non-local events = case where level memory is lacking |

`lookup : Map<K,V> × K → Option<V>` is the perfect textbook case: the operation must carry the key *downward* (inherited), compare against encountered keys (cross-level context), and *bring back up* the found value (synthesized). None of these three channels exists in the current homomorphic model. The note therefore produces, without seeing it, the requirements statement of the proposal: **the « Decision IR » must be founded on an attributed-grammar semantics over the diagram** — inherited + synthesized + level memory — of which DDD homomorphisms, folds and saturation are the three instances. This is the piece to write first, and it is absent from the note's plan.

---

## 8. Estimation and implementation plan: solid, but two major risks are missing from the table

The estimation part is reasonable and honest (phases 6/10 → 9/10, Sylvan/CUDD backend first, planner in stages, falsifiable A/B/C benchmark — very good). The risk table is incomplete:

1. **Missing the No. 1 scientific risk, identified by the previous analysis**: the cost model (when is the symbolic representation worthwhile). The note brushes against it (« AI will not invent the cost function ») without treating it as the project's bottleneck. The A/B/C benchmark measures it *a posteriori*; a *predictive* brick would be needed (cf. Solvsten & van de Pol 2023 on predicting the memory demands of operations).
2. **Missing the « Semantic DD explosion » risk** (§ 6.1 above): the whole architecture rests on recompilation from the semantic form; if that form is a monster, the architecture degrades into systematic reconstruction.
3. **Missing the memory/locality metrics** of the previous analysis: the proposed benchmark (nodes, memory, operation times) measures neither cache misses, nor table hit-rates (often dominant), nor placement cost — even though the note makes layout a level-3 « annotation ».
4. **Missing the saturation test**: the proposed shortcut (model `State / Set<State> / step / reachable`) is the right one, but it must include a model with **non-local events** — the only one that simultaneously puts to the test the cross-level channel and saturation by homomorphisms.

On the numbers themselves: the orders of magnitude (prototype 1–2 months, research system 3–6 months, framework 18–36 months) are plausible *for the engineering*; they underestimate the research (canonicality classified by constructor, attributed semantics, order alignment) on which the interest of the whole depends — and the note implicitly admits this point by concluding on the falsifying question. To keep: the discipline « an architecture that allows replacing the backend » and « do not debug seven layers at once ».

---

## 9. Synthesis — what we keep, what we correct, what we add

**To keep as-is**
- The three operation families (paths / across paths / closure) as the level-2 grid.
- `Canon(T, O_s, R)`: relative canonicality, never absolute.
- Order as an algorithmic resource; conversions (`convert`, `materialize`) as first-class nodes with explicit cost in the Physical IR.
- The default algebra by type.
- The staged implementation plan and the falsifiable A/B/C benchmark.

**To correct**
- The canonicality theorem: unique *relative to* (atom schema, decomposition, order, rules); classification bounded `Seq`/`Set`/`Map` = canonical vs open domains = non-canonical DDD. Rely on the antecedents: minimal automata (Seq), MSDD (Set), Queue BDDs (failure of unbounded canonicality), HSDD (Map/values).
- The Semantic DD: optional and never a systematic gateway (explosion risk).
- The planner: admit that with higher-order functions, the downstream is runtime; dynamic reordering remains a necessary fallback.
- The composition of different decompositions/permutations: to be treated as *the* central problem, not as a detail.

**To add (absent from the note)**
- Attributed semantics (inherited/synthesized/level memory) as the foundation of level 2 — it implements exactly the three operation families and repairs the homomorphism deficit identified by *Specification of Decision Diagram Operations* (2010). `lookup` = textbook case; non-local events = validation testbed.
- The predictive cost model as brick No. 1.
- Memory metrics (tables, cache, placement) in the benchmark.

**Strategic positioning**: this note makes the Geneva lineage (ΣDD, MSDD: Buchs/Hostettler) and that of polyDD (controllers) converge toward the same object: a layered library where the low layer is a generic backend (Sylvan/CUDD or extended polyDD), the middle layer the decision algebra (controllers + attributes), and the high layer the language. The middle layer is the contribution; the other two are known engineering. It is exactly « from less structured to more structured » — and each layer must require of the layer below only what it already provides (this is the layer-clarity criterion to respect in the code: `semantic/` does not know `backend/`).

---

## 10. References verified for this analysis

- Buchs, D., Hostettler, S. — *Sigma Decision Diagrams*, 2009 (origin of ΣDDs, Geneva group).
- Levi, L., Hostettler, S. — *Multi-Set Decisions Diagrams*, 2009 (direct antecedent of the `Set` layer).
- Godefroid, P., Long, D. — *Symbolic Protocol Verification with Queue BDDs*, LICS 1996 / STTT 1999. https://doi.org/10.1023/a:1008771008310 (antecedent — and canonicality failure — of the unbounded `Seq` layer).
- Couvreur, Encrenaz, Paviot-Adet, Poitrenaud, Wacrenier — *Data Decision Diagrams for Petri Net Analysis*, ATPN 2002. https://doi.org/10.1007/3-540-48068-4_8 (set semantics of paths; assumed non-canonicality).
- Hamez, Thierry-Mieg, Kordon — *Hierarchical Set Decision Diagrams and Automatic Saturation*, ATPN 2008. https://doi.org/10.1007/978-3-540-68746-7_16 (non-Boolean terminals = shared sub-DDs; saturation by homomorphisms).
- Miner & Ciardo — *Efficient Reachability Set Generation and Storage Using Decision Diagrams*, ATPN 1999. https://doi.org/10.1007/3-540-48745-x_2
- Hermanns, Meyer-Kayser, Siegle, Plateau — *Multi Terminal BDDs to Represent and Analyse Continuous Time Markov Chains*, 1999 (the multi-terminal function interpretation).
- van Dijk & Pol — *Sylvan: multi-core framework for decision diagrams*, 2016. https://doi.org/10.1007/s10009-016-0433-2
- Solvsten & van de Pol — *Predicting Memory Demands of BDD Operations Using Maximum Graph Cuts*, TACAS 2023 (brick of the predictive cost model).

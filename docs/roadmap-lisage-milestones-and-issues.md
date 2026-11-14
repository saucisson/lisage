# Roadmap: milestones and issues

Starting point: the GitHub repository **saucisson/project-template**, adapted for Rust. The project repository (suggested name: `lisage`) will host both the code **and** the documents produced during the conversation (imported into `docs/`, issue M0-3). The milestones take up and refine the sequencing of the proposal (§ 8); the visualization tool (**Reader**, described in the dedicated canvas) is integrated into each milestone rather than added at the end: it is the instrument for verifying the statements (C1, minimality per handle) and for producing figures.

## 1. Repository conventions

**Adapting the template to Rust** (what changes; everything else is kept):

| Template element | Rust adaptation |
|---|---|
| `ruff` + `mypy` (pre-commit, CI) | `rustfmt` + `clippy -D warnings` (+ `cargo-deny` for licenses/warnings) |
| `.python-version` | `rust-toolchain.toml` (pinned stable channel) |
| `pyproject.toml` | workspace `Cargo.toml` + `deny.toml` |
| pre-commit | kept: `gitleaks`, `rustfmt`, `clippy`, end-of-file, conflict markers |
| release-please | kept as-is (tag-based versioning) |
| GitHub Actions CI + `.gitlab-ci.yml` | same steps: fmt → clippy → test (`cargo-nextest`) → coverage (`llvm-cov`) → audit (`cargo-audit`) |
| `AGENTS.md`, `.vibe/`, `tools/notify` | kept (Vibe sessions, notifications) |

**Cargo workspace, one crate per layer** (proposal § 8):

```
lisage/
├── crates/
│   ├── lisage-lang/      level 1: types, reference interpreter (concrete syntax: surface decision, § 10)
│   ├── lisage-algebra/   level 2: extended controller, transductions, reductions
│   ├── lisage-plan/      profitability oracle, conversions, representation plan
│   ├── lisage-core/      level 3: pool, handles, GC, bi-plan, placement
│   ├── lisage-backend/   witnesses: sylvan | cudd, degraded mode
│   ├── lisage-bench/     differential, property-based, MCC, non-local events
│   └── lisage-liseuse/   the visualizer (trace, index, SVG/HTML rendering)
├── docs/                 the conversation documents (see M0-3)
└── bench-results/        bench outputs, exported figures
```

**Labels**: `area/core`, `area/algebra`, `area/lang`, `area/plan`, `area/bench`, `area/liseuse`, `area/docs`, `area/infra` ; `type/feat`, `type/test`, `type/doc`, `type/spike` ; `M0` … `M5` milestone labels ; and one label per falsifiable statement: `C1`…`C8`, `T1`…`T5`, `A5`, `L3`, `A6`. Each issue cites its statements in the body; each PR closes with `Closes #n` (template convention).

**Document mirror**: the canvases remain the source of truth during drafting; `docs/` is its versioned mirror (initial import in M0-3, resynchronization at each milestone). Target: `docs/lisage-proposal-the-attributed-framework-of-decision-diagrams.md`, `docs/specification-computing-attributed-transductions-reference-instance.md`, `docs/state-of-the-art-decision-diagrams.md`, `docs/golden-thread-example-n-philosophers-end-to-end.md`, `docs/critical-analysis-dd-ir-design-note-and-memory-locality.md`, `docs/critical-analysis-sdd-structural-orders-and-layered-library.md`, `docs/reader-visualizer-of-diagram-orders-and-operations.md`, `docs/roadmap-lisage-milestones-and-issues.md` (this document).

**Execution model: agent, human review, just-in-time discussion.** The code is written by an agent (Vibe Code), the design is clarified in the conversation, the human reviews. Three consequences for the plan. (1) **Estimates leave man-weeks**: the production of code, tests and documentation collapses; the real clock becomes threefold: availability of human review (the critical path), bench machine time (long PBTs, MCC corpus), and the pace of clarifying the open points in the discussion. Scientific risk (C1, unfold, T5) does not compress: falsification takes the time it takes. The planning unit is therefore the **cycle** (one agent work session closed by a human review), and the figures of § 8 remain indicative until the calibration of the first milestone. (2) **Just-in-time escalation**: the agent never blocks on an unresolved point (§ 10 below, open questions of the spec); it escalates it in the discussion, the answer becomes an ADR or a spec line, and work resumes. (3) **Per-milestone ordering discipline: interface → tests → implementation.** Phase 1, the interface: traits, types, bounds, error policy, ADR; a compiled and documented boundary is the ideal contract for an agent (the boundary test of M0-2 anchors it from the start). Phase 2, tests: the reference interpreter first (the oracle, deliberately naive and without DD), then the properties per statement (red PBT) and the differential harness. Phase 3, implementation: make it green, statement by statement. The discipline holds **per milestone**, not for the whole project: freezing the M3 interface before the lessons of M1 would be a return to waterfall; the order applies to the unit of work, just-in-time.

## 2. M0: setup (one cycle)

| Issue | Title | Content | Acceptance criterion |
|---|---|---|---|
| M0-1 | Repository from project-template | Template import, Python purge (ruff/mypy/.python-version), Rust setup (toolchain, pre-commit, release-please) | Green CI on the fmt/clippy/nextest/coverage/audit steps, with a dummy test |
| M0-2 | Workspace skeleton | The 7 crates, empty but with declared boundaries (unidirectional dependencies in `Cargo.toml`, commented with the layering criterion of proposal § 8) | `cargo nextest` and `clippy -D warnings` pass; a forbidden dependency is rejected by a boundary test |
| M0-3 | Document import | The conversation documents in `docs/`, relative links checked, one README per document (status: work in progress) | `docs/` complete, no broken anchors |
| M0-4 | Test charter | Triptych A/B/C described (naive / fixed-order DD / Lisage+planner), reference interpreter specified as the source of truth, `proptest` installed, naming convention `test_c<n>_*` per statement | A first dummy differential test passes in all three lanes |
| M0-5 | Bench foundation | `criterion` wired in; containerized MCC harness empty (the corpus arrives in M5, the tooling does not) | A micro-bench runs in CI and rejects time regressions |

## 3. M1: the verifiable core (milestone 1 of the proposal, 4 to 6 cycles)

**Summary**: `core/` minimal, reference instance `bounded Atom + Set + Map`, inherited + synthesized transductions, `lookup` as a test operation, `map/filter/exists/fold`, differential against the reference interpreter, Sylvan/CUDD witnesses on the classic BDD instance. Reader v0: trace + static diagram view. Execution order (discipline of the execution model, § 1): **interface** (M1-0), **tests** (M1-7, the oracle, then the red PBTs per statement), **implementation** (M1-1 to M1-6 made green), closure (M1-8 to M1-10).

| Issue | Title | Content | Statements |
|---|---|---|---|
| M1-0 | Core interface (phase 1: interface) | The interface traits: pool, key algebra, extended controller, `Observer`; `NodeId`/`Handle`; `Send`/`Sync` bounds (§ 10); error policy; ADR 0001+; boundary test extended to signatures; CUDD/Sylvan FFI spike in parallel (§ 10, before any dependency of the differential) | compile, `cargo doc`, green boundaries | contract |
| M1-7 | Reference interpreter (phase 2: tests) | The truth, in pure `lang/` without DD; A/B/C differential wired from the first commit; the PBTs per statement written **red** before the implementation | falsifiable |
| M1-1 | Arena + handles | `arena`, `NodeId`, hash-consing by key (controller, anchor, form); the handle table is keyed on the reduced form for this read | C1 |
| M1-2 | Reductions § 2.3 | R-share, R-collapse, R-default; confluence tested by PBT (`reduce∘reduce = reduce`, commutation of quotients) | C1 |
| M1-3 | Reference schemas | `Atom(D)` bounded, `Set(T)`, `Map(K,V)` bounded; universe-sweep compilation (one presence level per key, δ = absent, without marker) | Σ |
| M1-4 | Inherited/synthesized transductions | The two channels, the `φ_aggr` aggregation, memo fingerprints (op, node) | L3 |
| M1-5 | `lookup` | The test operation: synthesized = DD fragment (not a scalar), key carried downward, comparison in context | spec § 5 |
| M1-6 | Fold and semirings | `map/filter/exists/fold`; counting = fold over (ℕ, +, ×); declared construction merge (hash-consing = Bool choice) | A5, C7 |
| M1-8 | BDD witnesses | Controller configuration in classic BDD; differential against CUDD/Sylvan (identical reproduction), bench baselines | C6 |
| M1-9 | Instrumentation port | `Observer` trait (no-op by default) in `core/`; typed events; versioned JSONL trace serializer; Reader v0: static SVG rendering of the diagram, levels = keys | Reader |
| M1-10 | Philosophers demo | n = 3, 4, 5 in `map/filter/exists` + saturation via `fold`; figure exported from Reader; the 7 expected forms visible and verified | C8 |

**Milestone exit criteria**: null differential against the interpreter on the reference instance; C1 holds (uniqueness of handles, verified by PBT); the Reader diagram view shows the 7 forms of the philosophers and their multiplicities.

## 4. M2: expressiveness (milestone 2)

**Summary**: `Seq`/`Tag`, level memory, `saturate` with both policies, non-local events. Reader v1: the step-by-step trace.

| Issue | Title | Content | Statements |
|---|---|---|---|
| M2-1 | `Seq` and its two compilations | Repetition + end marker (key required); compilation by positions and by element enumeration; T5 hardened for enumeration (bounded by length, not by the universe) | T5 |
| M2-2 | `Tag` | The typed union: one level per useful edge, discriminant | C2 |
| M2-3 | Level memory | State stored between two visits of the same level; storage policy declared by the controller | T3 |
| M2-4 | `saturate`, two policies | Static by levels (Ciardo-style) and auto-detected (Hamez-style), as two instances of the same fixpoint semantics | T3 |
| M2-5 | Non-local events | Compilation of width-w transitions into level-memory transductions, poly(w) overhead | T3, C1 |
| M2-6 | Reader v1: step-by-step | Scrubber on the trace; colored channels (inherited, synthesized, memory); `MemoHit`/`MemoMiss`; saturation rounds; before/after differential with ghosts | Reader |

## 5. M3: order and plan (milestone 3)

**Summary**: complete `Order<T>`, conversions with predictable cost, analytical profitability oracle. Reader: the order view and kingdoms.

| Issue | Title | Content | Statements |
|---|---|---|---|
| M3-1 | `Order<T>` as a first-class value | Typed orders, π per type constraints in `Order<T>`, alignment as typing (two operands compose only under a common order) | C3 |
| M3-2 | Conversions | Reordering = converting, transductions with predictable cost; typed contract extended from orders to encodings (controller kingdoms) | C3 |
| M3-3 | Analytical profitability oracle | First predictive model: domain size, expected density, encoding cost, number of downstream queries; guardrail via differential | C4 |
| M3-4 | Reader: order view | O as a linear track or tree (vtree), controller card (the eight fields), kingdoms side by side, arrowed conversions with cost | Reader |

## 6. M4: locality and sharing (milestone 4)

**Summary**: two-plane pool, attribute co-location, cache metrics; activation of relative forms (shift) and of the generalized default (D-abs); measurement of the philosophers against the hierarchical SDD encoding. Reader: the memory and quotient views.

| Issue | Title | Content | Statements |
|---|---|---|---|
| M4-1 | Two-plane pool | Main and shadow planes, GC, profiled compaction; cache metrics in `bench/` | C5 |
| M4-2 | Relative forms | Activation of the shift (relative encoding, anchor) planned by the core schema from M1 on; plus the two extensions of spec 2.4.5: composed anchors (offsets named per fragment) and key transformations designated by the controller (action group + canonical section of the orbits; canonical instance: min = 0 normalization, argument n on the incoming edge; applications: unbounded counters, colors, convex regions; the canonical instance now has its test model: the sliding window, canvas second-golden-thread-example-the-sliding-window-protocol: the sequence base = the argument on the edge, the slide = the anchor displacement, the ISN = the canonical section, Wireshark as industrial precedent) | C8 |
| M4-3 | D-abs / D-trans | Generalized default: absorption over any subdomain, D^def designated by the controller | A6 |
| M4-4 | Philosophers benchmark | Measurement against SDD/HSDD: 7 expected nodes, O(1) + edge, constant left-handed ratio 1.2887, live peak, passes, firings; explicit falsification if the measurement says otherwise | C8 |
| M4-5 | Reader: memory and quotient views | Pools, GC, multiplicity badges (replicas), quotient classes (forms × anchors), naive/DD/Lisage comparison mode | Reader |

## 7. M5: scale and completeness (milestone 5)

**Summary**: complete language, higher-order functions, MCC corpus, learned oracle, installation of sentential SDDs, ADD/MTBDD instance. Reader: article figure export.

| Issue | Title | Content | Statements |
|---|---|---|---|
| M5-1 | Complete language + HOF | Level 1 complete; the planner admits a runtime downstream (degraded reordering as fallback) | C3 |
| M5-2 | MCC corpus | Containerized MCC 2011-2018 models; import stack for the corpus formats (PNML parsing, conversion to `lang/`: a parser of *external formats*, not of the Lisage language); complete A/B/C triptych; publication of the numbers | C3, C4 |
| M5-3 | Sentential SDDs | Completeness test: installation without ad hoc extension (vtree = structured order, tower of controllers as instance pattern) | C6 |
| M5-4 | ADD/MTBDD | Non-idempotent merge, table indexed by weights; compatibility differential as guardrail (open question no. 8 of the spec) | C7 |
| M5-5 | Reader: figure export | Publication mode: SVG/PGF clean for article layout, serif theme, auto captions from the channel legend | Reader |

## 8. Synthesis

| Milestone | Review cycles (indicative) | Targeted statements | Reader |
|---|---|---|---|
| M0 | 1 | none (foundation) | none |
| M1 | 4 to 6 | C1, L3, A5, C7 (start), C6 (start) | v0: trace + static diagram |
| M2 | 2 to 3 | T3, T5, C2 | v1: step-by-step trace |
| M3 | 1 to 2 | C3, C4 | order/kingdoms view |
| M4 | 2 to 3 | C5, C8, A6 | memory/quotient views |
| M5 | 3 to 5 | C3, C4, C6, C7 (completeness) | figure export |

A **cycle** = one agent work session closed by a human review. The compression compared to the original man-weeks comes from the execution mode (§ 1), but it is bought neither on the real clocks (availability of review, bench machine time, pace of clarification in the discussion), nor on scientific risk (C1, unfold, T5). Figures to be recalibrated after M1 on the cycles actually consumed.

The golden thread: **each milestone ends with a figure or a measurement published from Reader**. The tool is not a pedagogical luxury; it is the means to see C1 (two identical forms under different anchors do not merge), the saturation work (rounds, firings) and the sharing (multiplicities, O(1) + edge), which are the three things the text asserts and that the reader of an article will want to see.

## 9. Backlog (extension candidates, outside M0-M5)

- **Continuous δ** (open question 9 of the spec): region-key algebras, then intervals and DBMs; test instance: DDD-difference / CDD. The key contract (normal form, coverage, intersection) is declared on the controller side; the pool does not change. The fold becomes an integral: to be launched only after the ADD/MTBDD instance (M5-4), the two questions touching the same attribution field.
- **Coinductive** (candidate open question 10, cf. state of the art § 10): cycle node + quotient by bisimulation, outside C1. To be reopened only if a use case demands it.
- **Textual DSL + editor tooling** (if the surface is textual, decision § 10): its own workstream, to be launched after M5-1: concrete syntax specification (data: record/union/list/set/map declarations; operations: transductions, folds), parser (`lalrpop` or `chumsky`), LSP server (`tower-lsp`), linter (diagnostics = typing and admissibility, spec § 3, already specified), completion (name resolution + types), highlighting and VS Code extension (tree-sitter grammar, TextMate in v0). Tooling a syntax before it stabilizes is paying the debt twice; nothing in M0-M5 requires the textual surface (the milestones are driven through the API). The PNML parser of M5-2 is independent and does not wait for this one.
- **The didactic workshop (the graphical editor of paths)** (request of 10 November 2026): starting from a model and **paths of its state space** (a small whole model, or traces drawn from a large one), displayed bare, then **enable/disable each reduction one by one** (hash-consing, merge, collapse of the absent, relative form/anchor, shift, fold) and **play with the controller parameters** (order, absolute/relative encoding, kingdom, composed anchor): the diagram and its node count recompute live, the state space shrinks in real time. The goal is didactic: to make tangible the thesis "the diagram is a function of the controller", each paid reduction visible one by one (the 7 against 4n − 4 of the philosophers, the 4 against the infinity of the window, the left-handed ratio 1.2887 with the naked eye). It is the shuttle at human scale: the human plays the role of the LLM (order, anchoring hypotheses), the engine arranges and measures; it is also the foundation of demonstrations (Rennes table, Fete de la Science, MOOC, bookmark of the weaving, ring 3). Dependencies: no new parts (the core API, the Reader's `Observer` port, a standalone web front); unlike the Reader (read-only, it replays traces), the workshop **writes** the controller and re-asks: it is therefore launched after M3-4 (the order and the controller card exist), the two golden-thread examples serving as calibration benches.

### Inter-project tracks (opening session, 10 November 2026)

Eight directions crossing Lisage, petrichor and layered-data, ranked by proximity to the current plan. Each branches at least two projects and reuses a part already planned; none touches the pool.

- **The cartomancer**: the oracle reads the model. The LLM (petrichor's semantic reading) goes through the names and structure of the network and proposes a controller configuration (order, encoding, Set/Map compilations, kingdom); the analytical oracle measures, the LLM guesses, the differential corrects (M3-3, C3, C4). Falsifiable: the LLM does not predict better than the heuristic threshold, exactly clause C4. It attacks head-on the historical problem of variable ordering.
- **The shuttle**: the LLM proposes, the diagram arranges. The LLM emits hypotheses (symmetries, replica families, invariants), the engine verifies them by evaluation, and proven symmetries activate the shift (C8, M4-2). The correction never leaves the engine: the LLM is a hypothesis generator with zero confidence. Loop: guess, verify, exploit, measure in Reader.
- **The controller loom**: each layer of layered-data carries its compilation contract towards the pool; the controller is derived from the formalism. The tower of controllers (instance pattern of the proposition) becomes a theorem of the layers, and the installation of sentential SDDs (M5-3) is its executable test: write the SDD layer, derive the controller, zero *ad hoc* extension.
- **The reverse of the fabric**: counter-examples told. The fixpoint contains the counter-example; extract a minimal drawn path (re-explicitation of a single path, the price of A6 paid on a single branch), then the LLM tells it in the modeler's vocabulary (names, intentions). Assisted diagnosis complements petrichor's static analyses with the dynamic narrative.
- **The velvet**: the continuous δ vein told as a product: temporal networks (DBM keys, known convex closure), fold as integral (open question 9), stochastic (semirings of densities). Target QEST+FORMATS, strictly after M5-4, per the backlog.
- **Quantum embroidery**: beyond the non-idempotent merge of M5-4, semirings of complex amplitudes; the state of the art has the witnesses (LIMDD, Q-Sylvan). A long track, to be launched only if a use case arrives.
- **The forger**: an LLM that forges plausible and nasty models (false symmetries, almost perfect replicas, lying names) to falsify the oracle at scale (C4) and test petrichor's semantic reading against lying names. A source of bench families beyond MCC; an oracle-vs-forger tournament as a metric.
- **The ascent**: from the state space to the model. LLM-assisted network synthesis (region theory): the shuttle applied in reverse, the synthesized network is written in layered-data layers and verified by differential against the original space.

### Extended panorama (horizons, 10 November 2026)

Outside the trio, by rings. Nothing touches the Rennes sprint: these are horizons, not plan.

**Ring 1, other fabrics on the same loom** (application domains):
- **Process mining**: event logs → networks → state spaces, semantic conformance checking (petrichor reads the activity names); the Petri Nets CFP explicitly lists process discovery and conformance checking; an immediate industrial gateway.
- **Rule-based biology** (Kappa, metabolic networks): the combinatorial explosion of rules is the state explosion in its pure form; the shuttle's replica families are the agent symmetries; stochastic semirings; the CMSB community.
- **EDA**: the historical cradle of DDs (formal equivalence, property checking); reopened as an application domain, not as a venue (the arbitration of § 11 stands).
- **Planning and symbolic games**: factored MDP/POMDP (SPUDD lineage: ADD on factors), symbolic game solving; the probabilistic semiring of M5 is the door.

**Ring 2, other trades that weave the same way** (interdisciplinary bridges):
- **Probabilistic circuits**: PSDD and weighted model counting are already in the plan (M5-3); the horizon is the tractable probabilistic models community (UAI, NeurIPS): one engine, semirings.
- **Factorized databases**: Olteanu's factorization (query evaluation on decompositions) rhymes structurally with vtree and the controller (d-tree ≈ structured order); the planner of M3 is a factorization planner; the PODS/SIGMOD community. The deepest and least seen rhyme.
- **E-graphs and equality saturation**: the PLDI community's egg (in Rust) is the structural cousin of the hash-consing saturated pool; rewrite saturation and Ciardo saturation are two saturations; common ground to explore.
- **Attributed grammars and tree transducers**: the theoretical home of the framework (inherited/synthesized = cyclic and shared attribute grammars); and its most academic consequence: **the certified core**, C1/L3/A5 mechanized in Lean or verified in Creusot/Kani — a certified DD library does not exist yet.

**Ring 3, the factory and the city** (industry and society):
- **The symbolic auditor**: the event-sourced traceability of Reader + the LLM's narrative = explainable verification reports for certification (DO-178C, IEC 61508); the told counter-example becomes an audit artifact.
- **Digital twins and MBSE**: layered formalisms as a living formalization of SysML; simulation and safety analysis of industrial controllers on the same engine.
- **The weaving of state spaces** (specified on 10 November 2026; replaces the "Jacquard museum" entry). A real fabric, not a metaphor: the punched card is a decision structure, and the warp/weft are already the reading grid of the framework (inherited/synthesized). Protocol: **warp** = variables in order O (the record interleaving), **weft** = states, **point** = valuation (satin or its inverse depending on 1/0), **unreachable state** = absence in the weft, **shared form** = repeated pattern, **left-handed** = flaw of the pattern (the 1.2887 asymmetry visible to the naked eye). Three objects of the philosophers, at 10 wefts/cm: n = 3, 152 states: a 15 cm bookmark; n = 4, 23,168 states: a 23 m banner; n = 5, 536,754,176 states: **537 km of explicit fabric**, more than Paris-Lyon as the crow flies, while the same information fits in 7 patterns, a napkin. The pair "537 km against a napkin" makes C8 physical: the state explosion is an endless fabric, sharing is a game of cards. Feasibility: the digital jacquard looms of textile art schools are driven by a simple image file; the pipeline is Lisage computes, Reader exports the card (one more bitmap renderer after the SVG/PGF export of M5-5), the loom weaves (at 1 thread per variable the fabric is a ribbon; for a wide fabric, each variable becomes a band). Online services surveyed on 10 November 2026: three families. (1) Public weaving from image (real jacquard, not printing): WOVNS and Jacquard Works (fabric by the meter, MOQ 50 m: the n = 4 banner of 23 m fits in it), FiberArt, WovenPhotos, PictureWeave; Prodigi exposes the jacquard cover via a print-on-demand API, so the Reader → card → woven object pipeline can be automated end to end. Limit: these services weave pixels (brightness → point), not weaves. (2) Commissioning labs: TextielLab Tilburg (jacquard looms, open calls for projects, documented remote collaboration; weave control, hence the 1/0 valuation thread by thread and the left-handed flaw, lives at this level), independent workshops (jacquard portraits on commission). (3) Free tooling: AdaCAD (open-source jacquard CAD, Unstable Design Lab), documented TC2 export; integrable into Reader as one more rendering target. Partners to probe: a Lyon weaving school, the Musee des Tissus, Le 19M; the object can travel: bookmark on the demo table at the tool exhibition, Fete de la Science, MOOC in dress. Post-sprint by construction: its only engineering dependency is a renderer.

**Ring 4, the mirror** (the method itself):
- **The saucisson protocol as a research object**: agent-driven development (autonomous dev + reviewer agents, human at the day boundary) is a new methodology of which this project is the experiment: measure, publish (AI4SE, ICSE-SEIP communities), and build it from Lisage by Lisage, reflexively.

## 10. Completeness audit: points to clarify during development (2026-11-10)

Audit verdict: M0 and M1 are specified down to the issue level (statements, exit criteria, differential); open questions 1-9 of the spec are all assigned (Q1 → M3, Q2 → M2 to be measured, Q3 → M1 admissibility, Q4 → M1-3 verification, Q5 → M2-1, Q6 → M4-2, Q7 → M4-3, Q8 → M5-4, Q9 → backlog). Seven **engineering** decisions remain unresolved, absent from the documents; none blocks M0, and each is assigned to the milestone where measuring it is possible.

| Point | Open question | Milestone | Proposed decision |
|---|---|---|---|
| Interface concurrency | `Send`/`Sync` bounds of the pool and the computed-cache: freeze them in the signatures from M1-0 (interface phase), execution remaining single-threaded | M1-0 | freeze now (retrofitting a single-threaded interface costs more than the decision); effective parallelism pushed beyond M5, multi-core Sylvan serving as witness |
| Missing key in `lookup` | spec Q3: the `Option` of the result as default algebra, constant graft on continuations | M1-5 | make the case explicit in the issue (collapsable absent edge, to be integrated into admissibility § 3.3) |
| FFI CUDD/Sylvan | C/C++ bindings (compilation, CI containers, platforms): technical risk no. 1 of M1-8. Why bindings: the reference interpreter only sees the semantics, never the form; "identical reproduction" (C6) is measured in node count against an independent library (a homemade mini-BDD would share the same author's misunderstandings), and Sylvan adds a second independent witness carrying saturation and LDD. Out-of-process does not remove the risk: the libraries still have to be compiled; in-process buys the differential PBT (random function → two constructions → counts compared on the spot). External baselines condition the published measurements (M4-4, M5-2) | M1 | dedicated spike from the first week (the real question is the CI build); possible sequencing: CUDD alone in M1, Sylvan in M2 when saturation (M2-4, Hamez-style) requires it |
| Error policy | internal invariants (`core/`, `algebra/`) = asserts; errors of the `lang/` level = `Result` | M0-4 then M1-0 | two lines in the test charter, taken up as types in the core interface |
| Persistence | versioned dump format of diagrams; are handles stable across sessions? | M4 | to be specified with the two-plane pool (M4-1), the Reader's JSONL trace format not being a persistence format |
| Arithmetic of the probabilistic semiring | exact rationals vs `f64`: A5 (exact fold on lifted construction) requires the decision | M5-4 | exact by default (rationals), `f64` as a bench option; the guardrail is already written |
| ADR | freeze the structural choices of M1 (arena, `NodeId`, form encoding, Arc vs borrow) | M1-0 | `docs/adr/` convention from the first PR of the interface phase, imported in M0-3 with the documents |
| Language surface | the `lisage-lang/` skeleton announced a "parser" that no M0-M5 issue delivers; the concrete syntax is specified nowhere, and the editor tooling (LSP, linter, completion, VS Code highlighting) is absent from all milestones | M5-1 | decide: hosted Rust API (free lint and completion via rust-analyzer) or textual DSL (tooling in the backlog); neither blocks M0-M5, drivable through the API |

The two frozen backlog candidates (continuous δ, coinductive) block nothing by construction: the first waits for the ADD/MTBDD instance (M5-4), the second for a use case. The conjectures (C1 minimality, unfold) are not gaps: falsifiable by design, they are what M1 is built to test.

## 11. Publication plan (strategy noted on 10 November 2026)

Each milestone ends with a figure or a measurement published from Reader: the publication plan therefore follows the milestones, one article per group of successfully falsified contributions. Rule: submit only what the differential has already validated; the deadline never justifies submitting an untested statement.

| Article | Material (milestone) | Contributions | Target venue | Realistic timing |
|---|---|---|---|---|
| **PR: the framework applied to Petri nets (Rennes sprint)** | M0-M1 + the saturation slice of M2: attributed transductions, C1, static level-by-level saturation, philosophers in 7 forms, CUDD differential, Reader v0 figures; the chain of the three projects (layered-data: layered formalisms; petrichor: the Petri net tool; Lisage: the engine) | C1, T3 (static policy), C8 (philosophers compactness, without the O(1) + edge part which waits for M4) | **Petri Nets 2027** (Rennes: abstract 15 January, paper 22 January 2027); fallback: tool paper (10 p. + tool exhibition) | January 2027 |
| **P0: the reading grid** | no code: the great family (§ 4 of the state of the art) + the bestiary, every family = (controller, anchor, form) | the unifying reading, before falsification | **SYNASC** (BDD track, submission ~June, natural niche and entry barrier); **JFLA 2028** (October 2027, in French, for Σ and the language); long version as a survey after P1 | SYNASC 2027 |
| **P1: the core** | M1-M2: attributed transductions (inherited/synthesized/memory), C1, semirings (A5, L3), `lookup`, CUDD/Sylvan differential, Reader figures | C1, C2, C7 | **TACAS** (the audience of the DDD/SDD lineage, foundations + tool); fallback **ATVA** (~April); **CAV** if the canonicity classification angle dominates | TACAS 2028 (submission ~end of October 2027) |
| **P2: locality and saturation** | M4: shift, D-abs, O(1) + edge, philosophers (7 nodes, left-handed 1.2887), against SDD/HSDD | C5, C8 | **Petri Nets 2028** (theory 20 p., tools 10 p. + exhibition: Reader enters as a tool paper); **ACSD 2028** (the home of polyDD 2010) | submission ~January 2028 |
| **P3: completeness and scale** | M5: sentential SDDs without *ad hoc* extension, oracle on the MCC corpus, ADD/MTBDD merge | C3, C4, C6, C7 | **CAV** (the flagship paper of the completeness thesis); the MCC numbers also have their audience at Petri Nets (the competition lives there) | CAV 2029 (January 2029) |
| **Journal** | extension of P1 + P3 | all | **STTT** (the home of CESRBDD) or **FMSD** (the home of the previous generation) | 2029 |
| **Continuous δ vein** | backlog, after M5-4 | open question 9 | **QEST+FORMATS** (joint, time and stochastic vein); **HSCC** if the hybrid fragment opens | only if the backlog opens |

Dates noted on 10 November 2026, to be rechecked at each submission: TACAS 2027 closed on 29 October 2026; CAV 2027 closes on 20 January 2027 (too early for code: viable only for a position without implementation); JFLA 2027 closed on 16 October 2026 (Dinard); Petri Nets 2027 (Rennes, Inria): abstracts 15 January, papers 22 January 2027: **the target of the Rennes sprint, below**; HSCC 2027 closed on 5 November 2026. Revision of 10 November 2026: the "too early for code" was calibrated on a human pace; on the evidence of recent deposits (`layered-data` complete in a few days, `petrichor` scaffolded on 7 November and already 65 commits), the first window with code becomes Petri Nets 2027, TACAS 2028 remaining the target of the foundations paper, and P0 (SYNASC 2027, ~June) installing the reading grid between the two.

### The Rennes sprint (decision of 10 November 2026)

Three projects committed: **layered-data** (the layered formalisms: the Petri net derived from the hypergraph foundation, the philosophers as an instance; already delivered), **petrichor** (the Petri net tool: models, edges, runs; young, active), **Lisage** (the state-space engine via attributed DDs; to be built: M0-M1 + the saturation slice of M2). The regular paper (20 p. LNCS): attributed transductions applied to Petri nets, static level-by-level saturation, the compactness of the philosophers (7 forms), the C1 sharing visible in Reader, CUDD differential. **Anti-collision of claims**: PR carries the application (T3 static, philosophers compactness); P1 keeps the foundations (C1, C2, C7, `lookup`); P2 keeps the full locality (shift, D-abs, O(1) + edge, left-handed), which waits for M4.

Sequence (agent cycles, human review on the fly): weeks 1-2: M0 + M1-0 (interface) + reference interpreter + red PBTs + CUDD spike; weeks 3-5: M1 core (arena, handles, reductions, inherited/synthesized transductions, `lookup`, fold), green CUDD differential on the BDD instance, C1/L3/A5 hold; weeks 6-7: static saturation + philosophers n = 3 to 5 via the record interleaving; week 8: Reader v0, measurement freeze, abstract (15 January); weeks 9-10: writing from the documents, review, submission (22 January).

Scope cut to hold: CUDD as sole witness (Sylvan waits for M2-M4); philosophers only, no MCC, no `Seq`, no non-local events beyond the interleaving, no sentential SDDs, no ADD, no oracle (M3-M5 intact). **Guardrail (our submission rule): if on 15 January the differential is not green, the regular paper withdraws in favor of the tool paper** (10 p. + demo at the tool exhibition), which requires only the live philosophers demo and free access for the reviewers. The schedule bends; the statements do not bend.

Communities, to arbitrate: TACAS/CAV/ATVA = tool-oriented verification, where the DDD/SDD lineage lives; Petri Nets/ACSD = the saturation home and the polyDD/HSDD lineage; SYNASC = the BDD niche; STTT/FMSD = the journals that published CESRBDD and the previous generation; JFLA/GDR GPL = the French-speaking languages and software engineering community. DAC/ICCAD/DATE: set aside (EDA audience, different style), to be reopened only if the knowledge compilation angle (PSDD) ever dominates the agenda.

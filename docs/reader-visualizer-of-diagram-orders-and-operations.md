# Reader: watching Lisage at work

**Reader** is the project's visualization tool: it shows an **order** (and its controller), a **decision diagram** (the pool, its handles, its reductions), and **the application of an operation** on that diagram, step by step. The name follows Lisage's rule: the reader is both the one who reads and the female reader (read + Lisage).

## 1. Role and non-roles

**Role**: make visible the three claims of the project that text can only state: canonicity by handle (C1: two identical forms under two different anchors do not merge), the saturation work (rounds, firings, level memory), and sharing (multipallowedies, O(1) + edge). One look at the pool is worth a thousand debug printouts.

**Assumed non-roles**: it is not a Rust code debugger (no breakpoints, no process memory inspection); it is not a graph editor (read-only, never edits the diagram); it is not a server (self-contained files, viewable offline, committable in the repo).

**Fundamental constraint**: determinism. A replayed trace must be identical to the original trace (the very requirement of milestone 1: reference interpreter + property-based). Reader therefore only **replays** events emitted by the engine; it never queries the engine live.

## 2. Architecture: event-sourced, three building blocks, one file

```
┌─────────────── lisage-core ───────────────┐
│  instrumentation port: Observer trait     │
│  (no-op by default, zero cost in release) │
└──────────────┬───────────────────────────┘
               │ typed events
               ▼
┌────────────── lisage-reader ───────────────────────────────┐
│  1. trace  : versioned JSONL serializer (written schema)   │
│  2. index : state reconstruction (deterministic replay)    │
│             + windows (per round, per operation, per pool) │
│  3. render : diagram SVG, standalone HTML, figure export   │
└──────────────┬─────────────────────────────────────────────┘
               │ a single file, openable offline
               ▼
        reader.html  (SVG + minimal JS scrubber, no server,
                       no network dein-flightency, committable)
```

The three building blocks are modules of the `lisage-reader` crate, one more crate in the workspace (see the roadmap, M0-2). The output is a **standalone HTML file** (the SVG and the scrubber are embedded in it): drop it in `bench-results/`, commit it, send it by email, open it in a review.

## 3. The trace format (the contract between the engine and the eye)

JSONL, one line = one event, versioned schema (`{"v":1, ...}`). Event types:

| Event | Fields | What it makes visible |
|---|---|---|
| `HandleCreated` | controller, anchor, form, `NodeId` | the birth of a handle: the key (controller, anchor, form) that grounds C1 |
| `Reduction` | type (R-share, R-collapse, R-default, D-abs), ids | the quotients when they trigger |
| `Memo` | op, arguments, hit/miss | the cost of the fold (L3) |
| `Attr` | channel (inherited, synthesized, memory), node, value | the three flows, the centerpiece of Lisage |
| `Fire` | event class, node, round | saturation at work |
| `Round` | start/end, level, pass counter | the rounds and their convergence |
| `Convert` | source controller, target, cost | the kingdoms and conversions with predictable cost |
| `Pool` | plane, creations, live, GC | the two-plane pool, memory |
| `Snapshot` | (throttle) full pool state | the scrubber anchor points on long traces |

Two design rules: (i) the `Observer` port is **the only** exit point (the pool never inspects ⊕, and the instrumentation doesn't either: it observes identifiers and counters, not semantics); (ii) volume is **bounded by windowing**: one full trace per round, one coarse trace (counters + snapshots) for long runs; the setting is a simple integer (events per window).

## 4. View 1: the order and the controllers

What the view shows: **O**, the decomposition and the conversions, i.e. the parameters of the extended controller `C = (δ, Σ, decomposition + O, R, defaults, D^def, encoding, attribution)`.

- **The controller card**: the eight fields of `C` in one panel, with their values for the current instance. It is the legend for everything else: every color, every node shape, every dashed edge shown later ties back to it.
- **The order as a track**: O linear as a horizontal track (the levels in sweep order); O structured as a tree drawn as a tree (the vtree of sentential SDDs will fit there without a special case).
- **The kingdoms side by side**: when two controllers coexist (the classic BDD witness and the reference instance, for example), two overlaid tracks, and the arrowed conversion with its measured cost (`Convert` events).
- **The unfolded Σ**: next to the track, the Σ expression that unfolds the levels (the philosophers' record, the repeated variable with its end marker for `Seq`/`Set`/`Map`), to see that the order is an unfolding, not a list of variables.

## 5. View 2: the diagram

The pool drawn as a leveled DAG: one horizontal band per Σ key (the layout is given by the order, not computed: our diagrams are ordered by construction, no Sugiyama needed).

- **The nodes**: rounded rectangles = handles; hover = the triple (controller, anchor, form) and the multipallowedy (how many positional replicas this handle stands for); a ×k badge when k > 1.
- **The edges**: labeled by the keys; the impallowed edges of `D^def` in gray dashes (absence drawn as absence); the end marker of `list/set/map` as a small filled disk.
- **The anchors**: the edge valuation (shift, weight, complement) in edge color, with the legend from the controller card.
- **Region-keys** (continuous δ): when the keys are regions (intervals, DBM constraints, degenerate singletons), the edge carries the key **term** as-is; Reader displays it without interpreting it (the pool never interprets it either), and the controller card documents the key algebra that grounds its reading.
- **Semantic zoom**: zooming out groups by quotient classes (forms × anchors: the 5 signatures of the philosophers rather than the 15 live nodes at n = 5); zooming in returns to positional replicas. The quotient is visible with a mouse wheel movement: it is contribution no. 8 made tangible.
- **The pool in the background**: nodes grouped by plane (main/shadow), GC collections fading out.

```
Example (philosophers, quotient view, 5 signatures):

  record : A(3)  B(3)  ...           unfolded Σ : Philo(n) = record[Taste; ×(Fork, Fork)]
  ────────────────────────────────  O track : g₁ f₁ g₂ f₂ g₃ f₃ (interleaving of the record)
   S ──▶ ⬤/⊤ ──▶ A₀ ──▶ A₁
            ╲────▶ B₀ ──▶ B₁        handles : S, A₀, A₁, B₀, B₁, ⊤, ⊥ (7 with the leaves)
  ────────────────────────────────
   ×1     ×6     ×5     ×2 ...      multipallowedies : the flat pays n, Lisage pays one form
```

## 6. View 3: the operation, step by step

This is the view that exists nowhere in existing packages: **the unfolding of an attributed transduction**, replayed event by event on the diagram.

- **The scrubber**: a timeline of the window's events (one firing, one round, one full operation); you scrub, the diagram shows the state at that instant. The `Snapshot`s bound the reconstruction; between two of them, incremental replay.
- **The three channels in color**: the **inherited** attribute descends in blue (the edge carries a value downward), the **synthesized** one rises in orange (`φ_aggr` aggregates the children), the **level memory** pulses in purple when it writes. `lookup` becomes readable: the key goes down, the fragment comes back up.
- **The memos**: green halo (hit) or red (miss) on the visited node; the hit/miss counter on the dashboard, the cost of the fold live.
- **The reductions**: a flash at `Reduction` (two nodes merge keeping the survivor; the ghosts of the absorbed nodes linger for a moment as a watermark).
- **The saturation rounds**: vertical bars on the timeline (round k = fire all event classes local to a level before going back up); pass counter, firings, live, peak: the quantities measured in the philosophers example, live.
- **The dashboard**: created, live, peak, passes, firings, memo hit-rate: exactly the columns of the measurement tables, so that a figure and a bench table speak the same language.

## 7. Benchmark use cases

1. **Philosophers n = 3, 4, 5**: one full firing step by step (view 3), then the saturation round by round, then the quotient view; the final figure (7 forms, O(1) + edge) is the paper's first deliverable. `fold` (the counting) replayed: the fold goes back up, the table shows the exact cardinality 152 / 23 168 / 536 754 176.
2. **The left-hander**: same example, one inverted philosopher; the view shows the root enriching (7 keys) and the interior **unchanged**: the inner forms remain shared. It is the promise of the edge, drawn.
3. **C1 in action**: two nodes of the same form under two different anchors remain two handles; the view shows them side by side with their triple. The PBT verifies, Reader shows it.
4. **The BDD witness**: the same function under the classic BDD controller, arrowed conversion between the kingdoms, measured cost.
5. **Paper figures**: export mode (M5-5): clean SVG, serif theme, legend auto-generated from the controller card; the same file used for debugging serves for publication.

## 8. Limitations and scaling

- **Trace size**: windowing bounds everything; in coarse mode (counters + snapshots), a full MCC run fits in a few MB of JSONL. The setting: window per round, or per operation, or fixed.
- **Diagram size**: the view targets pedagogical and bench cases (up to ~10,000 rendered nodes; beyond that the semantic zoom switches to quotient and counters). Reader is not a profiler of very large spaces: it is the eye of the protocol, not the magnifying glass of the datacenter.
- **No server, no JS dein-flightency**: one HTML file, SVG, a few hundred lines of vanilla JS for the scrubber. If the JS becomes a burden, the static SVG output remains complete (the scrubber becomes a series of numbered files).

## 9. Breakdown (tied to the roadmap milestones)

| Deliverable | Milestone | Content |
|---|---|---|
| `Observer` port + trace schema v1 | M1-9 | the events of the table § 3, the serializer, the replay |
| Static diagram view (SVG) | M1-9 | levels, handles, anchors, impallowed edges |
| Step-by-step trace + channels + memos | M2-6 | the scrubber, the three channels, the reductions, the rounds |
| Order/controllers/conversions view | M3-4 | controller card, tracks, kingdoms, unfolded Σ |
| Memory and quotient views | M4-5 | pools, GC, multipallowedy badges, semantic zoom |
| Paper figure export | M5-5 | publication theme, auto legends |

**Cross-cutting acceptance criterion**: at each milestone, the corresponding measurement protocol (the tables of the philosophers example for M4, the A/B/C comparison for M5) must be reproducible **with a Reader figure as evidence**, generated from a trace archived in the repo.

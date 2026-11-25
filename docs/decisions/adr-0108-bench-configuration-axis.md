# ADR-0108: Bench configuration axis — two tiers, selection grids, tool idiom

- Status: accepted (accepted by the human owner on 2026-11-24)
- Date: 2026-11-24

> Drafted alongside the BENCH-MC milestone issues (labels `area/bench`,
> `jalon/BENCH-MC`, milestone BENCH-MC #30). Extends ADR-0102 (local Docker
> witnesses). Agents draft ADRs inside a pull request; only the human owner
> accepts them. Accepted by the human owner on 2026-11-24.


## Context

Campaigns 1 to 4 compared lisage against external witnesses with every tool
in its default configuration. Meanwhile lisage carries a real reduction
profile axis (`Reductions` in `crates/lisage-core/src/workshop.rs`), the
confrontation runner hardcodes `Reductions::without_collapse()`, and the
orchestrator's only knob is `oxidd_env`. Campaign-4 therefore answers
"default lisage vs default witnesses", not "lisage at its best vs each tool
at its best" — the question the Model Checking Contest actually asks, since
tools there are configured by their own engineers before the run.

Campaign-4 also carries the methodological lesson: libDDD was measured as a
raw BDD package, an out-of-idiom usage that measured a disguise of the tool,
not the tool. Any future campaign must not repeat that with any witness.

Finally, the overload studies (issues #347-#354) now provide a
per-model-class table, making a class-based corpus possible instead of an ad
hoc instance pool, with falsifiable predicted gains for lisage.

## Options considered

1. **Keep default-only comparisons** — rejected: a default-configured
   witness is not the witness; verdicts would flatter lisage wherever
   another tool's knob pays.
2. **Full configuration exploration (8 knobs per tool)** — rejected: the
   combinatorics do not fit the campaign budget (~2 h of measures on M1);
   the anti-flattery rule needs equality and ex-ante declaration, not
   exhaustive search.
3. **Two-tier comparison with selection grids** (chosen): every tool gets a
   small, equal, pre-declared grid; the default configuration always stays
   visible as the reference tier.
4. **Model transformations inside the nominal duel** — rejected for the
   duel: structural reductions are a different comparison axis; they get
   their own track with its own verdict, so neither side is measured with
   someone else's machinery.

## Decision

1. **Two tiers, equal budgets.** Tier D (default configuration) is the
   reference and continuity line; tier A (adapted configuration) carries
   the headline verdict. Every tool gets the same grid size (4 named
   configurations), declared ex ante, validated by the human before the
   campaign, then frozen. Selection runs 1 repetition; a gap under 20%
   between the two best triggers a second; finals run 5 repetitions with
   medians (the repository standard). The measured configuration name rides
   the existing `variante` key.

2. **Idiom rule.** Each tool is used in its own idiom: native
   representation, native automatic heuristics, automatic saturation
   included when the tool has one. Every grid entry is a mode the tool
   itself exposes (or, for driver-side settings, a usage the tool's
   ecosystem documents as recommended) and cites its reference (doc,
   paper, or CLI option). The campaign-4 libDDD-as-BDD measurement is the
   named anti-pattern: measuring a tool disguised as another tool measures
   the disguise, not the tool. The `|S|` + CTL cross-check protects
   correctness; the idiom rule protects fairness. Both are auditable in
   the committed campaign plan.

3. **lisage grid = published ablation.** The lisage grid is the four real
   reduction profiles (`ALL`, `NONE`, `no-collapse`, `no-default`),
   measured at 5 repetitions each and published in full, winner included:
   "does this reduction pay, on this class of models?" becomes audible
   with data.

4. **Unified PNML protocol.** Every witness receives the PNML file itself
   and builds its own state space, then answers three bounded MCC-like
   exams: deadlock (EF), reachability, safety (AG). Cross-checks on state
   counts and CTL verdicts across tools and configurations are mandatory;
   divergences are recorded, not repaired. For oxidd and sylvan, which do
   not load PNML natively, the CTL evaluation lives in a driver written
   once and shared between them (same algorithm, two backends); the
   verdict is attributed to the library + driver pair, documented as such.
   its-tools keeps its native engine: that is the subject of the
   comparison, not a bias to neutralize.

5. **Witness set.** The nominal duel: its-tools, oxidd, sylvan.
   - its-tools is rebuilt locally from the official chain (MCC-drivers +
     ITS-Tools-MCC + official converter jar), every component pinned by
     SHA; measurements call `BenchKit_head.sh` with
     `BK_TOOL`/`BK_EXAMINATION`, never the published Flask image.
     Documented fallback: LIP6 CI static binaries pinned by URL + SHA256.
   - CUDD returns as a **historical anchor**: tier D only, nominal corpus,
     5 repetitions, outside grids, tier A, track 2, and the headline
     verdict. It is the flat-BDD classic any reviewer expects, with a
     falsifiable prediction inherited from campaign-4 (lisage 3-5x ahead,
     lowest memory). Its driver is adapted to the unified PNML protocol
     like the others.
   - libddd stays out: its idiomatic evaluation (DDD/SDD hierarchy,
     homomorphisms, automatic saturation) is precisely the pinned
     its-tools chain, which is in the duel.

6. **Nominal duel stays at the decision-diagram level.** its-tools runs
   without structural reductions in the duel (its native hierarchy and
   saturation included). Model transformation is track 2, with its own
   verdict: its-tools `+red` (structural reductions + SMT) vs lisage's
   per-class transformations (per the overload studies) vs oxidd/sylvan
   fed the residual model exported by the its-tools reducer.

7. **Memory cap 1 GB, hard rule.** Per measurement, every tool. Witness
   containers capped by `docker run --memory 1g`; the lisage process
   bounded the same way (containerized if needed, for perfect symmetry);
   `rss_max_ko` always reported; a breach is a recorded "over budget"
   verdict in the aggregates, not a crash. An instance that does not fit
   1 GB for a duel tool is demoted to the high campaign; it is a data
   point, not a blind spot.

8. **Class-based corpus.** One model per model class from the overload
   studies: philosophers (I), petN (II), Kanban (III; SwimmingPool and
   erk stay as regression charges), FMS++ shared crane (IV), 2D torus (V),
   referendum (VI), N-queens as included negative control (VI). Two v1
   instances per family (S, L); XL instances go to the high campaign.
   Admission rule: slowest duel tool in default config under ~20 s of
   blocked measure; 600 s per-run guard. The overload-studies predicted
   gains are the falsifiable baseline for lisage.

9. **Results live in the repository.** Every campaign commits its raw
   JSONL lines, aggregates, the full selection journal (grids plus every
   tuning attempt and tie-break), a reproduction README with the machine
   header, and a falsifiable `REPORT.md` under
   `bench-results/<run-id>/`, arriving by pull request like everything
   else. Nothing lives only on the measurement machine.

## Consequences

- The headline becomes "best configuration of lisage vs best
  configuration of each tool", which is the MCC question; defaults stay
  visible for continuity with campaigns 1 to 4.
- The lisage ablation is complete (4 profiles x 5 repetitions) rather than
  winner-only.
- Budget: ~892 runs ≈ 2h35-2h55 of measures on M1 (host 16 GB DDR4, WSL2
  VM 7.7 GiB by default, 1 GB cap per measure, strictly sequential), one
  evening. Tool installation is paid once, never budgeted.
- Negatives accepted: 1-repetition selection can pick a noisy winner
  (mitigated by the tie-break and the 5-repetition finals); grids
  restrict exploration to 4 knobs per tool; the shared driver carries
  driver-written CTL for oxidd/sylvan; lisage can lose the adapted duel,
  which is publishable either way (tool paper positioning).
- The ADR-0102 witness pattern gains a second-generation user (its-tools)
  without changing its rules: local only, never in CI.

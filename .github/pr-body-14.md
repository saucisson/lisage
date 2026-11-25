## Summary

M1-8 (témoins BDD, énoncé C6) en **mode dégradé pur Rust** : configuration du contrôleur en BDD classique dans `lisage-core::bdd` (arcs 0/1 explicites, pas de R-default, R-collapse + R-share seuls), un **mini-BDD témoin indépendant** dans `lisage-bench::temoin_bdd` (table unique propre, écrit exprès pour le différentiel), une **PBT C6** (nombre de nœuds identique + sémantique identique sur les 2^n affectations, fonctions booléennes aléatoires, n ∈ 2..=8), une **ligne de base de bench** (BDD n=12, table LCG déterministe, criterion en CI) et un **spike documentaire** `docs/spike-ffi-cudd-sylvan.md`. Le témoin externe CUDD/Sylvan (FFI) est une nouvelle dépendance : question `needs-human` structurée posée sur l'issue #14, aucune dépendance ajoutée ici.

## Issue closure

Relates to #14 (M1-8). L'issue reste ouverte avec le label `needs-human` : la partie FFI CUDD/Sylvan (vraie reproduction à l'identique contre une bibliothèque indépendante) attend la décision humaine sur la dépendance.

## Acceptance criteria mapping

| Criterion | Where it is satisfied | Evidence |
| ---------- | --------------------- | -------- |
| C6 (reproduction à l'identique, mode dégradé) | `lisage-bench/tests/temoins.rs` : PBT arène BDD classique vs mini-BDD témoin, comptes de nœuds égaux + évaluation conforme à la table de vérité | `cargo nextest run --all-targets` : 76/76 |
| Configuration BDD classique | `lisage-core/src/bdd.rs` : `BddController` (FINGERPRINT 3, `default_payload` = None), `BddKeys` (cardinalité 2, pas de δ), `compiler_bdd`/`compter_noeuds`/`evaluer` | tests unitaires `lisage-core/tests/bdd.rs` |
| Lignes de base de bench | `lisage-bench/benches/micro.rs` : `bdd_classique_n12_compter` (assertion interne arène == témoin) | `make check` (le bench tourne sous nextest) |
| Spike FFI | `docs/spike-ffi-cudd-sylvan.md` | — |

## Testing evidence

- `make check` - OK : 76 tests passés, 2 ignorés (rouge M1-7), clippy `-D warnings` OK, `cargo fmt --all` OK.
- `cargo nextest run --all-targets` - 76 passed, 2 skipped.
- Non exécuté : comparaison contre CUDD/Sylvan réels (dépendance non ajoutée, escaladée sur #14).

## Judgement calls

- **Pas de dépendance FFI** : la liste d'escalation AGENTS.md interdit toute nouvelle dépendance sans `needs-human` ; le différentiel C6 est livré contre un mini-BDD interne indépendant (même si « même auteur » affaiblit C6 — documenté dans le spike), et la vraie question FFI est escaladée sur #14.
- **Convention des niveaux** : racine au niveau 0, niveau ℓ = variable ℓ, bit ℓ de l'affectation = variable ℓ (cohérent avec `evaluer` et le compilateur de schémas).
- **Comptage des nœuds** : nœuds atteignables depuis la racine, terminaux compris, même convention des deux côtés du différentiel.

## Split proposal (ADR-0019)

The diff is 541 added lines across 11 files, marginally over the ~500-line
budget. Proposed split if a strict split is required: this PR keeps the
BDD-classique configuration + témoin mini-BDD + PBT C6 (the C6 evidence),
and a follow-up PR would carry the bench baseline (`bdd_classique_n12_compter`)
and the spike doc `docs/spike-ffi-cudd-sylvan.md`. Kept together here because
the baseline's internal assertion (arène == témoin) is what makes the bench
self-checking.

## Known limitations

- Le témoin mini-BDD partage le même auteur que l'arène : C6 « fort » (CUDD/Sylvan) reste à établir après la réponse `needs-human` sur #14.
- Pas de transduction sur le BDD classique dans cette PR (compter/évaluer seulement).

## Checklist

- [x] No constitutional file touched (le spike est un document de travail sous `docs/`, pas un ADR).
- [x] No new dependency added.
- [ ] `Closes #n` présent — volontairement absent : l'issue #14 reste ouverte (`needs-human` sur la partie FFI).
- [x] Full `make check` run on this branch.

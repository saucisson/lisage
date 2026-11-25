## Summary

M1-10 (démo philosophes, énoncé C8) : `lisage-liseuse::philosophes` construit l'espace atteignable R_n pour n = 3, 4, 5 en encodage plat intercalé (clés `2i` = pc_i, `2i+1` = left_i), avec énumération de référence des états (mots circulaires sans deux `eat` adjacents). Vérifications : |R_3| = 20, |R_4| = 56, |R_5| = 152 par pli `(ℕ,+,×)` (égalité DD == énumération), `filter`/`exists` sur la projection « mangeurs » de R_n, les 7 formes au niveau automate (5 cellules légales + 2 terminaux), et la figure SVG de R_5 exportée depuis la Liseuse (`bench-results/philosophes-n5.svg`, déterministe, commitée).

Cette PR livre aussi la réponse à l'issue #14 (M1-8 — témoins BDD, C6) : CUDD en conteneur Docker local, hors CI (décision humaine 2026-11-10, ADR-0102).

## Issue closure

Closes #16
Closes #14

## Acceptance criteria mapping

| Criterion | Where it is satisfied | Evidence |
| ---------- | --------------------- | -------- |
| C8 : \|R_3\|=20, \|R_4\|=56, \|R_5\|=152 | `test_c8_cardinaux_r3_r4_r5` : pli DD == énumération == fil rouge | `cargo nextest run` : 86/86 |
| n = 3,4,5 en map/filter/exists + saturation via fold | `construire_r_n` (union = saturation de construction), `compter` (fold), `test_filter_cellule_zero_mange`, `test_exists_sur_r_n` | tests d'intégration `crates/lisage-liseuse/tests/philosophes.rs` |
| 7 formes visibles et vérifiées | `test_sept_formes_contre_r_n` : 5 cellules légales + 2 terminaux (⊤ atteignable, ⊥ non atteignable), chacune apparaissant dans R_n | 86/86 |
| Figure exportée depuis Liseuse | `test_figure_svg_r5_exportee` → `bench-results/philosophes-n5.svg` (commitée, comparée au rendu) | fichier présent, rendu déterministe |
| C6 local (hors CI) : différentiel contre CUDD réel | `docker/cudd/` (image `lisage-cudd:latest`, CUDD 3.0.0 pinné par SHA) + `crates/lisage-bench/tests/cudd_local.rs` (`#[ignore]`, hors CI) | `docker build` OK, `cudd_count` vérifié ; test local exécuté manuellement |

## Testing evidence

- `make check` - OK : 86 tests passés, 3 ignorés (dont `cudd_local` hors CI), clippy `-D warnings` OK, `cargo fmt --all` OK.
- CI verte sur le head (3 check runs success).
- Non exécuté en CI : le différentiel CUDD (nécessite Docker, hors CI par décision humaine).

## Judgement calls

- **7 formes au niveau automate, pas au niveau DD** : en encodage plat M1, les signatures de nœuds croissent avec n (replicas positionnels) ; la version « 7 nœuds indépendants de n » exige les formes relatives (M4-2). On vérifie ici les 5 cellules légales de l'automate minimal + les 2 terminaux (⊤ atteignable, ⊥ non atteignable depuis R_n).
- **filter/exists sur la projection « mangeurs »** : les prédicats de présence (M1-6) ne voient pas les valeurs ; la projection mangeurs (clé i présente ssi pc_i = eat) rend la présence discriminante.
- La figure SVG est écrite par le test dans `bench-results/` et comparée au fichier commité (déterministe : pas de diff en CI).
- CUDD en Docker local (ADR-0102) : pas de dépendance C/C++ en CI, le mini-BDD interne reste le témoin de CI.

## Known limitations

- Pas de saturation par événements (take/release/hungry) : R_n est construit directement par énumération + union (la saturation événementielle est M2-4).
- Figure statique unique (R_5) ; pas de scrubber (M2-6).
- Le test `cudd_local` échoue tant que le BDD interne n'est pas réduit à l'identique de CUDD (écart arène 7 vs CUDD 5) — `#[ignore]` documente le différentiel local.

## Checklist

- [x] No constitutional file touched (ADR-0102 ajouté pour le répertoire `docker/`).
- [x] No new dependency added (CUDD hors CI, en conteneur Docker local).
- [x] `Closes #16` and `Closes #14` present in the body.
- [x] Full `make check` run on this branch.

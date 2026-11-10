## Summary

M1-10 (démo philosophes, énoncé C8) : `lisage-liseuse::philosophes` construit l'espace atteignable R_n pour n = 3, 4, 5 en encodage plat intercalé (clés `2i` = pc_i, `2i+1` = left_i), avec énumération de référence des états (mots circulaires sans deux `eat` adjacents). Vérifications : |R_3| = 20, |R_4| = 56, |R_5| = 152 par pli `(ℕ,+,×)` (égalité DD == énumération), `filter`/`exists` sur la projection « mangeurs » de R_n, les 7 formes au niveau automate (5 cellules légales + 2 terminaux), et la figure SVG de R_5 exportée depuis la Liseuse (`bench-results/philosophes-n5.svg`, déterministe, commitée).

## Issue closure

Closes #16

## Acceptance criteria mapping

| Criterion | Where it is satisfied | Evidence |
| ---------- | --------------------- | -------- |
| C8 : \|R_3\|=20, \|R_4\|=56, \|R_5\|=152 | `test_c8_cardinaux_r3_r4_r5` : pli DD == énumération == fil rouge | `cargo nextest run` : 85/85 |
| n = 3,4,5 en map/filter/exists + saturation via fold | `construire_r_n` (union = saturation de construction), `compter` (fold), `test_filter_cellule_zero_mange`, `test_exists_sur_r_n` | tests d'intégration `crates/lisage-liseuse/tests/philosophes.rs` |
| 7 formes visibles et vérifiées | `test_sept_formes_automate` : 5 cellules légales + 2 terminaux, chacune apparaissant dans R_n | 85/85 |
| Figure exportée depuis Liseuse | `test_figure_svg_r5_exportee` → `bench-results/philosophes-n5.svg` (commitée) | fichier présent, rendu déterministe |

## Testing evidence

- `make check` - OK : 85 tests passés, 2 ignorés, clippy `-D warnings` OK, `cargo fmt --all` OK.
- Non exécuté : la version « 7 nœuds du DD » des formes (encodage relatif = jalon M4-2, hors M1).

## Judgement calls

- **7 formes au niveau automate, pas au niveau DD** : en encodage plat M1, les signatures de nœuds croissent avec n (replicas positionnels) ; la version « 7 nœuds indépendants de n » exige les formes relatives (M4-2). On vérifie ici les 5 cellules légales de l'automate minimal + 2 terminaux, chacune apparaissant dans R_n.
- **filter/exists sur la projection « mangeurs »** : les prédicats de présence (M1-6) ne voient pas les valeurs ; la projection mangeurs (clé i présente ssi pc_i = eat) rend la présence discriminante.
- La figure SVG est écrite par le test dans `bench-results/` (déterministe : pas de diff en CI) et commitée.

## Known limitations

- Pas de saturation par événements (take/release/hungry) : R_n est construit directement par énumération + union (la saturation événementielle est M2-4).
- Figure statique unique (R_5) ; pas de scrubber (M2-6).

## Checklist

- [x] No constitutional file touched.
- [x] No new dependency added.
- [x] `Closes #16` present in the body.
- [x] Full `make check` run on this branch.

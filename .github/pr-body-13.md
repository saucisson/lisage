## Summary

M1-6 (1/2) : le pli et les semi-anneaux (spec § 5, A4/A5/C7) dans `lisage-core::pli`. Trait `SemiAnneau` avec `Naturel` (ℕ,+,×) et `BoolOu` (Bool,∨,∧) ; transduction `Pli<S>` (fold) avec expansion-comptage A4 ; `compter` (count) ; `union` = merge de construction Bool idempotent (le hash-consing est ce choix, A5). `phi_aggr` reçoit le niveau des fils (A4). Correction de `reduire` (M1-2) : un niveau à défaut ne collapse plus quand deux arcs explicites mènent à une cible ≠ D_δ — le collapse n'était pas préservant (spec § 2.2) et rendait le comptage inexact.

Closes #13 (partie 1/2 : pli, comptage, union ; map/filter/exists en PR suivante)

## Acceptance criteria mapping

| Criterion | Where it is satisfied | Evidence |
| ---------- | --------------------- | -------- |
| Pli sur semi-anneau, H trivial, terminal = base | `pli.rs` `Pli<S>`, `psi` | `test_pli_boolou_non_vide` |
| Comptage = pli sur (ℕ,+,×), A4 (cardinaux des niveaux) | `Pli<Naturel>`, `multiplicateur` on-path, `compter` | `test_count_niveau_collapsed_a4_exemple_spec` (exemple spec § 5 : count = 2), PBT `test_c1_differentiel_count_voie_a_vs_dd` |
| Merge de construction déclaré (hash-consing = Bool) | `union` + doc A5 | `test_union_merge_construction_bool` |
| Deux algèbres nommées (A5) | doc module `pli.rs` : construction Bool (union) vs requête `S` (paramètre du pli) | revue |
| Correction R-collapse préservant la sémantique | `reduction.rs` couvre | tests M1-2 (confluence) toujours verts + PBT count |

## Testing evidence

- `make check` - OK (63 tests, 2 ignorés rouges M1-7)
- `cargo clippy --all-targets -- -D warnings` - OK
- `cargo fmt --all` - OK
- Non exécuté : `cargo audit`, CI réelle avant push.

## Judgement calls

- **Correction de `reduire` (M1-2)** : `ν(ℓ, {présent ↦ X, absent ↦ X})` avec X ≠ D_δ ne collapse plus. Le collapse n'était pas préservant pour la sémantique ensemble de chemins (spec § 2.2 : les arcs absents valent δ, mais deux arcs *explicites* sont des chemins distincts) ; sans la correction, `union` perdait des membres et le comptage A4 était inexact (cas limite A5). Tests de confluence M1-2 et tests de schémas toujours verts.
- Facteur A4 « on-path » : le niveau valeur d'une clé absente est hors chemin (facteur 1 pour la présence sautée, |Val| pour une valeur sautée).
- `union` énumère toutes les valeurs d'un niveau sans défaut (niveau valeur).

## Known limitations

- `map/filter/exists` (deuxième famille et gardes) : PR suivante (feat/13b).
- Pli pondéré (multiplicités de chemins) : exige une construction levée (ADD), hors M1 (A5, roadmap M5-4).

## Checklist

- [x] No constitutional file touched.
- [x] No new dependency added.
- [x] `Closes #13` present in the body (clôture en deux PR).
- [x] Full `make check` run on this branch.

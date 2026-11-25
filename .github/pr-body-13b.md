## Summary

M1-6 (2/2) : les opérations à canal hérité seul (spec § 4) dans `lisage-core::operations`. `Existe` (pli `(Bool,∨,∧)`, verdict résolu au terminal sur le chemin), `Filtre` (garde : membres rejetés → `ε(false)`, règle A2 dans `φ_out`), `Image` (réétiquetage des atomes, schéma `Atom`). Le canal hérité porte l'ensemble des clés présentes sur le chemin. Différentiels voie A vs voie DD : `exists` en booléen, `filter`/`map` par égalité structurelle des nœuds (C1).

Closes #13

## Split proposal on the ~500-line PR budget (ADR-0019)

This PR adds 677 lines, above the ~500 budget. Proposed split, if the
human wants a strict sub-500 PR:

- **PR-B1** (`feat/13b-core`, ~340 lines): `crates/lisage-core/src/operations.rs`
  + `lib.rs` — the three transductions (`Existe`, `Filtre`, `Image`) and helpers.
- **PR-B2** (`feat/13b-tests`, ~340 lines): `crates/lisage-core/tests/operations.rs`
  + `crates/lisage-bench/tests/operations.rs` — unit tests and differential PBTs.

The single-PR form is kept because the tests are the acceptance evidence
for the operations; the split above remains available on request.

## Acceptance criteria mapping

| Criterion | Where it is satisfied | Evidence |
| ---------- | --------------------- | -------- |
| `exists` = pli `(Bool,∨,∧)`, verdict sur le chemin | `operations.rs` `Existe` | `test_existe_predicat_exemple_spec`, PBT `test_c1_differentiel_exists_voie_a_vs_dd` |
| `filter` = garde (image réciproque), sortie canonique | `Filtre`, règle A2 | `test_filtre_garde`, PBT `test_c1_differentiel_filter_voie_a_vs_dd` (C1 structurel) |
| `map` = image/réétiquetage | `Image` | `test_image_reetiquetage_atomes`, PBT `test_c1_differentiel_map_voie_a_vs_dd` (C1 structurel) |

## Testing evidence

- `make check` - OK (69 tests, 2 ignorés rouges M1-7)
- `cargo clippy --all-targets -- -D warnings` - OK
- `cargo fmt --all` - OK
- Non exécuté : `cargo audit`, CI réelle avant push.

## Judgement calls

- Prédicats de **présence** uniquement : bien définis sur le diagramme réduit. Un verdict dépendant des valeurs sur un niveau `val(k)` collapsé exige le T-EXPAND (§ 6.2, livré pour `lookup`) — documenté comme limitation, hors périmètre de cette PR.
- Règle A2 ajoutée à `Filtre::phi_out` : si tous les fils mènent à `ε(false)`, émettre `ε(false)` (sinon les sous-arbres tout-rejetés restaient non réduits sur les niveaux sans défaut).

## Known limitations

- Verdicts sur valeurs de niveaux `val(k)` collapsés : T-EXPAND à brancher (cf. `lookup`, § 6.2).

## Checklist

- [x] No constitutional file touched.
- [x] No new dependency added.
- [x] `Closes #13` present in the body.
- [x] Full `make check` run on this branch.

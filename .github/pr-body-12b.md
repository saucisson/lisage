## Summary

M1-5 (2/2) : la transduction `lookup(k)` (spec § 6) dans `lisage-core::lookup`. Un pool de sortie dédié (`LookupController`, payload `Option<u64>`, D_δ = `ε(None)` = clé absente) ; canal hérité `Mode::{Cherche, Extrait, Stop(v)}` (spec § 6.3) ; canal synthétisé = fragment de DD (arcs du niveau `val(k)` vers les terminaux `ε(Some(v))`) ; `φ_out` ré-explicite le niveau valeur par T-EXPAND + greffe quand R-collapse l'a supprimé (spec § 6.2, § 6.4). `SchemaKeys::new` devient public.

Closes #12

## Acceptance criteria mapping

| Criterion | Where it is satisfied | Evidence |
| ---------- | --------------------- | -------- |
| Modes cherche/extrait/stop, clé portée vers le bas | `lookup.rs` `Mode`, `phi_disp` | `test_lookup_cles_presentes`, `test_lookup_cle_absente` |
| Fragment de DD synthétisé | `Transduction::S = BTreeMap<u64, NodeId>`, `psi`/`phi_aggr` | assertion `fragment.contains_key(&1)` |
| T-EXPAND + greffe sur niveau valeur collapsé | `phi_out` branche T-EXPAND, `greffer` | `test_lookup_niveau_valeur_collapsed_t_expand` |
| Différentiel voie A (interprète) vs voie DD | `lisage-bench/tests/lookup.rs` PBT | `test_c1_differentiel_lookup_voie_a_vs_dd` |

## Testing evidence

- `make check` - OK (56 tests, 2 ignorés rouges M1-7)
- `cargo clippy --all-targets -- -D warnings` - OK
- `cargo fmt --all` - OK
- Non exécuté : `cargo audit`, CI réelle avant push.

## Judgement calls

- Cas collapsé : aucun `Stop` émis, la valeur vit dans le DD ré-explicité (T-EXPAND) ; le fragment synthétisé reste vide (testé explicitement).
- Mémo de la greffe par valeur `v` : les terminaux greffés dépendent de `v`, une mémo partagée serait incorrecte.

## Known limitations

- `Set(Map)` et l'union de fragments multi-maps attendus en M1-6 (#13).

## Checklist

- [x] No constitutional file touched.
- [x] No new dependency added.
- [x] `Closes #12` present in the body.
- [x] Full `make check` run on this branch.

## Split proposal on the ~500-line PR budget (ADR-0019)

This PR adds 582 lines, above the ~500 budget. Proposed split, if the
human wants a strict sub-500 PR:

- **PR-A** (`feat/12b-core`, ~280 lines): `crates/lisage-core/src/lookup.rs`
  + `lib.rs` + `schema.rs` (`SchemaKeys::new`) — the transduction itself, no tests.
- **PR-B** (`feat/12b-tests`, ~300 lines): `crates/lisage-core/tests/lookup.rs`
  + `crates/lisage-bench/tests/lookup.rs` (unit + differential PBT, incl. the
  collapsed T-EXPAND case) + WORKLOG entry.

The single-PR form is kept because the tests are the acceptance evidence
for the core and the excess is tests/doc-comments, not logic; the split
above remains available on request.

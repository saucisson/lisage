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

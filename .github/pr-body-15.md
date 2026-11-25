## Summary

M1-9 (port d'instrumentation, Liseuse v0) : le trait `Observer` et les événements typés existaient déjà (M1-0) ; cette PR livre les deux briques manquantes dans `lisage-liseuse` — le **sérialiseur de trace JSONL versionné** (`{"v":1,...}`, une ligne par événement, écrit à la main, zéro dépendance) avec un `JsonlObserver` collecteur, et le **rendu SVG statique du diagramme** (niveaux = clés en lignes horizontales, placement déterministe, terminaux en bas, arcs étiquetés).

## Issue closure

Closes #15

## Acceptance criteria mapping

| Criterion | Where it is satisfied | Evidence |
| ---------- | --------------------- | -------- |
| Trait `Observer` no-op par défaut dans core | `lisage-core::observer` (M1-0, inchangé) | — |
| Événements typés | `TraceEvent` (M1-0) sérialisés 1:1 en JSONL | golden tests `test_serialiser_evenement_golden` |
| Sérialiseur JSONL versionné | `lisage-liseuse::trace` : `TRACE_VERSION = 1`, `serialiser_evenement`, `JsonlObserver` | `test_trace_jsonl_versionnee_et_deterministe` |
| Liseuse v0 : SVG statique, niveaux = clés | `lisage-liseuse::svg::rendre_svg` | `test_svg_statique_deterministe`, `test_svg_structure_equilibree` |

## Testing evidence

- `make check` - OK : 80 tests passés, 2 ignorés, clippy `-D warnings` OK, `cargo fmt --all` OK.
- Non exécuté : rendu HTML autonome + scrubber (Liseuse v1, M2-6) ; pas de dépendance ajoutée (JSONL hand-rollé, pas de serde).

## Judgement calls

- JSONL sérialisé à la main : `serde` aurait été une nouvelle dépendance (liste d'escalation) — les événements ne portent que des entiers et des enums, l'échappement reste correct.
- `JsonlObserver` clonable (`Arc<Mutex<…>>`) : permet de lire la trace après avoir donné l'observateur au pool.
- Le SVG v0 rend le sous-diagramme atteignable depuis une racine donnée (déterministe), pas le pool entier.

## Known limitations

- Pas de scrubber ni de HTML autonome (v1, M2-6) ; pas de fenêtrage de la trace.
- Le SVG est statique : une seule image par diagramme.

## Checklist

- [x] No constitutional file touched.
- [x] No new dependency added.
- [x] `Closes #15` present in the body.
- [x] Full `make check` run on this branch.

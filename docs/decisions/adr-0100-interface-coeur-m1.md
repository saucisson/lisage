# ADR-0100 — Interface du cœur de M1 (état : **accepted** 2026-11-11, décision humaine reçue en session needs-human)

> Brouillon rédigé dans la PR de M1-0 : un ADR projet se *draft* dans une PR
> (numéroté à partir de 0100) ; seul l'humain l'accepte. La plage 0001-0099
> appartient au template.

## Contexte

M1-0 (issue #6) fige l'interface du cœur : traits `Pool`/`KeyAlgebra`/
`Controller`/`Observer`, types `NodeId`/`Handle`/`Anchor`/`Level`/`Form`,
bornes `Send`/`Sync`, politique d'erreurs (feuille de route § 10).

## Décisions proposées

1. **Placement des contrats** : identifiants et traits vivent dans
   `lisage-algebra` (le niveau 2 voit le port « stocker/récupérer un nœud par
   clé », proposition § 8) ; `lisage-core` implémente le port et expose
   `Observer`/`CoreError` en ré-exportant les types du contrat. Le graphe du
   workspace (test de frontières) interdit `algebra → core`.
2. **Arena + `NodeId` en ordre de création, jamais déplacés** (proposition § 6).
3. **Forme** : `Form::Node { level, arcs: BTreeMap<Key, NodeId> }`, arc par
   défaut stocké comme les autres ; nœuds possédés par l'arena, référencés
   par `NodeId` (pas d'`Arc` partagé en M1).
4. **Bornes `Send`/`Sync` figées maintenant** sur les quatre traits :
   exécution mono-thread en M1, parallélisme au-delà de M5.
5. **Erreurs** : invariants internes `core`/`algebra` = `assert!` ;
   frontières = erreurs typées (`PoolError`, `CoreError`, `LangError`).
6. **Logging** : narration par **événements DEBUG structurés** à champs
   nommés, épinglés par des tests (décision humaine du 2026-11-11, session
   needs-human — le contrat constitutionnel « logging replaces comments »
   s'applique). Aucune dépendance de log ajoutée pour l'instant (liste
   d'escalation) : tant qu'aucune crate de log n'est autorisée, l'émission
   passe par le canal `Observer` existant. La migration des modules existants
   (doc-comments + tests) vers les événements DEBUG est progressive, module
   par module.

## Conséquences

- Test de frontières étendu aux signatures : bornes vérifiées dans les
  sources (lisage-bench) et à la compilation (lisage-core).
- C1 se teste par PBT sur les handles (contrôleur, ancre), pas sur les nœuds
  nus (spec § 2.4.3).

## Alternatives écartées

- Contrats dans `core` : impossible sans inverser le graphe du workspace
  (changement d'architecture, escalation).
- `Arc<…>` partagé pool/transductions : `NodeId` suffit et reste simple en M1.

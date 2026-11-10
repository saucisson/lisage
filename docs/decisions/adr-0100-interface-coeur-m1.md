# ADR-0100 — Interface du cœur de M1 (état : **draft**, needs-human)

> Statut : brouillon rédigé par l'agent développeur dans la PR de M1-0,
> conformément au contrat (`docs/agents/developer.md` : un ADR du projet se
> *draft* dans une PR, numéroté à partir de 0100 ; seul l'humain l'accepte).
> La plage 0001-0099 appartient au template.

## Contexte

M1-0 fige l'interface du cœur (issue #6) : traits `Pool`, `KeyAlgebra`,
`Controller`, `Observer` ; types `NodeId`/`Handle`/`Anchor`/`Level`/`Form` ;
bornes `Send`/`Sync` ; politique d'erreurs. Feuille de route § 10 demandait
de figer les choix structurels de M1 dès la phase d'interface.

## Décisions proposées

1. **Placement des contrats** : les types d'identifiants et les traits
   (`Pool`, `KeyAlgebra`, `Controller`) vivent dans `lisage-algebra`, le
   niveau 2 qui *voit* le port « stocker/récupérer un nœud par clé »
   (proposition § 8) ; `lisage-core` implémente le port (`Arena`) et expose
   `Observer` + `CoreError`, en ré-exportant les types du contrat. Le graphe
   du workspace (test de frontières) interdit `algebra → core` ; placer les
   contrats dans `core` rendrait les transductions d'`algebra` incapables de
   les nommer.
2. **Arena + `NodeId` en ordre de création, jamais déplacés** : le plan du
   contenu est adressé par contenu (hash-consing), les identifiants par ordre
   de création (proposition § 6).
3. **Encodage des formes** : `Form::Node { level, arcs: BTreeMap<Key, NodeId> }`
   — l'arc par défaut δ(ℓ) est stocké comme les autres ; R-default le rend
   implicite. Pas de `Arc` vs borrow tranché ici : les nœuds sont possédés
   par l'arena, référencés par `NodeId` (copie, `Hash`).
4. **Bornes `Send`/`Sync` figées maintenant** sur `Pool`, `KeyAlgebra`,
   `Controller`, `Observer` (roadmap § 10) : exécution mono-thread en M1,
   parallélisme effectif au-delà de M5, Sylvan multi-cœur servant de témoin.
5. **Politique d'erreurs** : invariants internes de `core/`/`algebra/` =
   `assert!` ; frontières = erreurs typées (`PoolError` dans le contrat,
   `CoreError` dans `core`, `LangError` + `Result` dans `lang/`).
6. **Logging** : aucune dépendance de log n'est ajoutée (liste
   d'escalation) ; la narration vit dans les doc-comments et les tests qui
   les épinglent, en attendant un ADR dédié si un crate de log est un jour
   approuvé.

## Conséquences

- Le test de frontières s'étend aux signatures : bornes `Send`/`Sync`
  vérifiées dans les sources (lisage-bench) et à la compilation (lisage-core).
- Tout handle est canonique pour sa lecture (contrôleur, ancre) : C1 se
  teste par PBT sur les handles, pas sur les nœuds nus (spec § 2.4.3).

## Alternatives écartées

- Contrats dans `core` : impossible sans inverser le graphe du workspace
  (changement d'architecture, escalation).
- `Arc<…>` partagés entre pool et transductions : borrow checker + canonicité
  par handle rendent `NodeId` plus simple et suffisant en M1.

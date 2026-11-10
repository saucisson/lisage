# Spike FFI CUDD/Sylvan (M1-0, en parallèle — avant toute dépendance du différentiel)

*Feuille de route § 10, point « FFI CUDD/Sylvan » : risque technique n° 1 de
M1-8. Ce document est le résultat du spike ; aucune dépendance C n'est ajoutée
au workspace (liste d'escalation : toute nouvelle dépendance exige un
`needs-human`).*

## Pourquoi des liaisons FFI

L'interprète de référence ne voit que la sémantique, jamais la forme. La
« reproduction à l'identique » (C6) se mesure en nombre de nœuds contre une
bibliothèque indépendante : un mini-BDD maison partagerait les malentendus
du même auteur. Sylvan ajoute un second témoin indépendant (saturation, LDD).
L'in-process achète le PBT différentiel (fonction aléatoire → deux
constructions → comptes comparés sur place).

## Recette de build évaluée

| Bibliothèque | Source | Build | Conteneur CI |
|---|---|---|---|
| CUDD | https://github.com/ivmai/cudd (release 3.0.0) | `./configure && make` (C, autotools) | `debian:stable` + `build-essential`, ~2 min |
| Sylvan | https://github.com/trolando/sylvan | `cmake` + `make` (C++, dépend de GMP/MPIR) | idem + `libgmp-dev`, ~5 min |

Liaisons Rust prévues : `cudd-sys` / `sylvan-sys` (crates `-sys` existantes,
build via `cc`/`cmake` crate) ou liaisons maison via `extern "C"` sur un
sous-ensemble minimal (manager, `bdd_and`, `bdd_or`, `bdd_restrict`,
`bdd_nodecount`, `bdd_satcount`). Préférence : liaisons maison minimales —
moins de surface, pas de dépendance `cmake`.

## Risques relevés

1. **Build en CI** (le vrai risque) : compilation C/C++ dans GitHub Actions,
   plateformes (macOS arm64 pose problème à Sylvan/GMP). Mitigation : job
   dédié `witness-ffi` optionnel, le cœur restant 100 % pur Rust.
2. **Sémantique exacte** : ordre des variables, complément d'arête, gestion
   du terminal — la reproduction à l'identique exige de figer l'ordre et de
   compter les nœuds hors terminaux, convention CUDD.
3. **Séquencement** (roadmap § 10) : CUDD seul en M1, Sylvan à M2 quand la
   saturation l'exige.

## Décision du spike

- Le différentiel de M1-8 tourne contre un **témoin BDD interne indépendant**
  (mini-BDD Bryant pur Rust dans `lisage-backend`, mode dégradé) : zéro
  dépendance, PBT différentiel en CI dès M1.
- Le câblage FFI réel (CUDD d'abord) est un **follow-up bloqué sur
  escalation** : `cudd-sys` est une nouvelle dépendance → `needs-human`
  commenté sur l'issue #14. Le conteneur CI et la recette ci-dessus sont
  prêts à l'emploi dès l'accord.

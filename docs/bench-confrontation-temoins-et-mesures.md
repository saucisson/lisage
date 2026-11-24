# Confrontation lisage / témoins : outils, charges et mesures

Protocole du harnais `make bench-confrontation` : mesurer lisage face
aux outils externes de diagrammes de décision, en temps **et** en
mémoire, sur des charges identiques. Décision cadre : ADR-0102 — les
témoins externes tournent dans des conteneurs Docker **locaux**, hors
CI ; ce document et le harnais sont l'exécution de cette décision
humaine du 10 nov. 2026.

## Périmètre

Trois charges aujourd'hui, une grille qui s'étend par fragments (voir
« Feuille de route ») :

| charge | variante | entrée | vérification épinglée |
|---|---|---|---|
| `alea-lcg <n>` | BDD | table de vérité LCG déterministe (mêmes constantes que la ligne de base M1-8), n = 4…24 | satisfactions = poids de la table (n=4 → 7, n=12 → 2074) |
| `philosophes <n> <chemin>` | MDD | R_n du fil rouge, fichier d'états étendus **partagé** généré par l'orchestrateur | \|R_n\| : 20, 56, 152, 416, 1136 pour n = 3…7 |
| `fenetre <taille> <nb_bases>` | MDD (lisage : encodages absolu/relatif) | paires (base, position), générées en interne par chaque outil | taille × nb_bases chemins (10 bases × 4 → 40) |

La charge `alea-lcg` est le booléen pur (BDD classique) ; la charge
`philosophes` est le fil rouge n° 1 en schéma fini ; la charge
`fenetre` est le fil rouge n° 2 avec l'axe encodage — absolu et
relatif sont deux diagrammes lisage **différents** pour le même objet
logique (seuls les nœuds partagés diffèrent, jamais le dénombrement).

## Témoins

| outil | version épinglée | variante(s) | conteneur |
|---|---|---|---|
| CUDD | `fc30c18f8db2e7d03d13241038b61eb0056a734e` (tag cudd-3.0.0) | BDD | `docker/cudd/` (`lisage-cudd:latest`) |
| Sylvan | `47136cfb69a4e7d111c6632b5aa4651f41ef7f79` | BDD parallèle | fragment 2 |
| libDDD | `b775aed3e8405a8906b10f937f6dbcc4a4b8831c` | DDD/SDD (école de Paris) | fragment 3 |
| Meddly | `22ca8dab1f231106e08e341c3f32e55fd9dea73d` | MDD, saturation | fragment 4 |

Chaque témoin embarque son **propre** binaire de charge (même protocole
que le runner lisage, `cudd_bench` pour CUDD) : une charge, un
processus, une ligne JSON sur stdout. Le DDD-différence (intervalles,
DBM) n'a pas de témoin externe maintenu : il reste mesuré par le
différentiel interne lisage (documenté, pas comparé — l'absence est un
résultat, pas un oubli).

## Protocole de mesure

Le bloc mesuré est **identique chez tous les outils** : lecture de
l'entrée + construction du diagramme + comptage de vérification.
L'initialisation de la bibliothèque (manager CUDD, lace/Sylvan, domaine
Meddly) est volontairement **hors** du bloc : ce n'est pas la charge
comparée, et elle masquerait les petites tailles.

- **Temps** : horloge monotone interne au runner (`CLOCK_MONOTONIC` en
  C, `Instant` en Rust), en millisecondes.
- **Mémoire** : RSS maximale du processus, `VmHWM` de `/proc/self/status`
  en Rust, `ru_maxrss` (getrusage) en C. Linux seulement, documenté :
  les exécutions locales et les conteneurs des témoins sont Linux.
- **Répétitions** : chaque case (outil × charge × paramètres) est
  répétée dans un **processus frais** — le pic RSS d'une répétition
  n'hérite jamais de la précédente. Le rapport donne la médiane et
  l'étendue (min/max) ; les lignes brutes sont dans `mesures.jsonl`.
- **Équité mono/multi-thread** : lisage et CUDD sont mono-thread par
  défaut ; Sylvan (fragment 2) sera rapporté **séparément** en
  workers=1 et multi-worker — jamais une seule colonne mixte.

Chaque runner rend exactement une ligne JSON, ordre des clés épinglé
par test (`crates/lisage-bench/src/confrontation.rs` et le test
`confrontation` de `lisage-bench`) :

```json
{"outil":"…","variante":"…","charge":"…","parametres":{…},
 "encodage":"…","temps_ms":…,"rss_max_ko":…,"noeuds":…,"verifie":…}
```

(`encodage` absent hors fenêtre ; chez les témoins, l'encodage booléen
de chaque outil est celui documenté dans son binaire de charge.)

## Vérification croisée

Deux gardes, dans l'orchestrateur (`scripts/confrontation.py`) :

1. **Épingles indépendantes** : le fichier d'états philosophes est
   généré par l'orchestrateur et son cardinal comparé aux pins du fil
   rouge à la génération ; chaque ligne rendue est comparée aux pins
   connus (|R_n|, poids LCG, taille × nb_bases).
2. **Cohérence inter-outils** : pour chaque (charge, paramètres), tous
   les outils — y compris les deux encodages lisage de la fenêtre —
   doivent rendre le **même** `verifie`. Un désaccord arrête le bench :
   un nombre faux ne devient jamais un graphique.

`noeuds` n'est **pas** comparé entre outils : le nœud d'un BDD CUDD,
d'un MDD lisage et d'un DDD n'ont pas la même granularité ; seule la
tendance par rapport à la grille (croissance en n ou en nb_bases)
se lit entre colonnes, jamais la valeur brute.

## Reproduire

```sh
docker build -t lisage-cudd:latest docker/cudd
make bench-confrontation
# ou une sélection :
make bench-confrontation CONFRONTATION_OUTILS="lisage,cudd" \
     CONFRONTATION_CHARGES="philosophes" CONFRONTATION_REPETITIONS=7
```

Sorties dans `.confrontation/<horodatage>/` (git-ignoré) :
`scratch/` (fichiers d'états partagés), `mesures.jsonl` (lignes brutes),
`rapport.md` (une table par charge, médiane/étendue).

## Limites

- Les temps absolus ne se comparent **pas entre machines** ; seul le
  rapport lisage/témoin sur la même exécution est signifiant.
- CUDD et lisage n'ont pas le même ordre de variables/encodage : la
  comparaison mesure des constructions de complexités comparables,
  pas des implémentations de la même structure. Les ordres choisis
  sont documentés dans chaque binaire de charge.
- La grille actuelle est volontairement petite (elle doit rester
  exécutable en local, hors CI) ; l'axe taille est le premier
  paramètre d'extension.

## Feuille de route des fragments

1. **Socle + CUDD** (ce fragment) : runner lisage, binaire `cudd_bench`,
   orchestrateur, protocole, cible make.
2. **Sylvan** : docker/sylvan/, axe workers (mono/multi rapportés
   séparément).
3. **libDDD** : philosophes et fenêtre en DDD/SDD de l'école de Paris
   (homomorphismes, saturation pour la reachability) ; `alea-lcg` sans
   objet (booléen).
4. **Meddly** : philosophes et fenêtre en MDD (forêts MULTI_TERMINAL,
   saturation) ; même grille, mêmes épingles.
5. Hors périmètre assumé : ZDD (CUDD sait le faire, la charge booléenne
   n'exploite pas la canonicité zéro-supprimée — à ouvrir avec une
   charge clairsemée dédiée), SDD UCLA (sententiels, ADD), DDD
   intervalles (différentiel interne lisage uniquement).

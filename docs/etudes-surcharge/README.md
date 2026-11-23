# Études « surcharge », parties I à VII

Archive des sept études expérimentales consacrées à la question : **peut-on faire
mieux avec des annotations de surcharge sur les diagrammes de décision de lisage ?**
appliquées à des familles de modèles repérées dans le catalogue du Model Checking
Contest (Diffusion2D-like, Referendum, NQueens). Destinée au dépôt
(`docs/etudes-surcharge/`, issue ARCH-1) : chaque étude est reproduite par son banc,
et les résultats JSON sont régénérés à l'identique (bancs déterministes).

## Contenu

```
etudes/   01..07 : les sept études, copies exactes des documents publiés
bancs/    etude5.py, etude6.py, etude7.py : bancs Python (parties V, VI, VII)
bancs/    results5.json, results6.json, results7.json : résultats régénérés
```

- `etudes/` : les fichiers portent l'en-tête YAML d'origine (`name`, `title`,
  `type`) ; c'est une métadonnée de document, sans effet sur le contenu.
- Les bancs écrivent leurs résultats à côté d'eux-mêmes (`os.path` relatif) : ils
  tournent tels quels dans n'importe quel dossier du dépôt.
- Les résultats des parties I à IV ne sont pas regénérés ici : leurs bancs
  historiques (etude1-4.py) n'ont pas été reconstruits, seuls les résultats
  numériques figurent dans les études 01 à 04.

## Les lois de la série, une ligne chacune

1. **L'ancrage seul ne suffit jamais** (I) : l'écart plat/pool se joue sur la
   fenêtre et les coutures, pas sur le choix de l'ancre.
2. **L'écart explose sur les langages filtrés mondiaux** (II) : petN ×49 à n=6,
   pool polynomial (556) contre plat super-linéaire (27 462).
3. **Arcs valués et grappes monnayent la densité des workflows** (III-IV) : le
   Θ(K²) du FMS était un artefact d'ordonnancement, pas une fatalité.
4. **La couture, pas la symétrie, porte le gain** (V) : tore-pressé garde le
   plat du tore mais triple ses classes intérieures.
5. **Quand la garde ou la requête est globale, la fenêtre bornée n'est pas
   chère, elle est fausse** (VI) : 20 à 40 % de faux positifs sur les N-reines,
   types incorrects dès N=7 sur le référendum sans multiplicités.
6. **La portée des gardes dans l'ordre des variables décide de tout, et le
   contrôleur peut la lire statiquement** (VII) : chaque nœud du plat est un
   motif de mémoire de coupe ; égalité parfaite sur 328/328 niveaux, cinq
   verdicts historiques retrouvés sur cinq par le classifieur.

## Tables clés

### V : grilles 2D n×n, séries complètes (tore, cylindre, plan, tore-pressé)

| variante | plat n=2..5 | pool n=2..5 | A/B à n=5 |
|---|---|---|---|
| tore | 8 / 39 / 225 / 1159 | 11 / 43 / 158 / 300 | 17 / 283 |
| cylindre | 8 / 43 / 241 / 1407 | 13 / 52 / 174 / 299 | 17 / 282 |
| plan | 8 / 32 / 95 / 249 | 22 / 107 / 277 / 519 | 34 / 485 |
| tore-pressé | 8 / 39 / 225 / 1159 | 29 / 143 / 535 / 978 | 51 / 927 |

Le plat du tore-pressé est identique au tore (la lettre de surcharge en (0,0)
ne crée aucun nœud supplémentaire) ; son pool intérieur triple (3×17) : le
pression monnaye des classes, pas de l'espace. |S| n=6 par matrice de transfert
(imprimée au lancement de etude5.py) : tore 2 406 862, cylindre 3 557 711.

### VI : mémoire (extrêmes)

| famille | classement |
|---|---|
| référendum N=10 | \|S\|=59 049, plat=10, plat+requête=51, compteur=66, pool multiset 3 945, pool set 3 563, 205 classes ambiguës dont 34 décision-ambiguës, 5 valeurs de compte dans une classe |
| N-reines n=7 | \|S\|=16 870, plat(lignes)=15 180, plat(diagonales)=14 220, pool 290 (saturé dès n=5), erreurs fenêtre 39,9 % (annulées seulement à rayon N-1), empreintes 16 648 (quasi injectives) |

### VII : verdicts du détecteur

| famille | verdict détecté | accord avec I-VI |
|---|---|---|
| anneau C_n | fenêtre (coupe plafonnée à 4 dès n=6) | oui |
| grille plan | fenêtre (fins de lignes [8, 13, 13, 13, 2]) | oui |
| grille tore | fenêtre + couture (fins de lignes [8, 61, 112, 50, 2]) | oui |
| référendum | compteur (coupe vide, requête comptage-seuil) | oui |
| N-reines | registre (coupe 17 / 59 / 265 / 1146, croissance ×4 par pas) | oui |

Égalité théorème (nœuds du plat = motifs de mémoire de coupe) : **328/328
niveaux**. Contre-hypothèse « projections sur cellules encore lues » : égalité
partielle seulement (44 à 54 % sur grilles et N-reines, 83 et 100 % sur les
familles triviales) : c'est bien la mémoire de coupe, pas la projection.

## Reproduire

Tout est déterministe (Python 3.10+, aucune dépendance) :

```
cd bancs
python3 etude5.py tore 5         # puis cylindre 5, plan 5, tore-presse 5
python3 etude6.py referendum 10
python3 etude6.py nqueens 7
python3 etude6.py portees 6      # puis portees 7
python3 etude7.py
```

Quelques secondes chacun (RSS max 46 Mo). Chaque banc reconstruit
ex nihilo l'espace d'états, l'asserte par invariants (aucun conflit énuméré,
solutions complètes exactes, |S| par matrice de transfert) puis sauvegarde
dans son results*.json. Les valeurs affichées dans `etudes/` ont été
produites par ces mêmes bancs et se régénèrent à l'identique.

## Conventions

- **plat** : MDD positionnel exact dans l'ordre naturel des cellules ;
- **pool** : nombre de classes de signatures locales (fenêtre bornée),
  jamais un moteur déployable, un indicateur de structure ;
- **coupe I(p)** : ensemble des cellules futures directement interdites par
  les cellules occupées du préfixe à la coupe p ;
- **requête** : transition de conclusion agrégative (référendum : majorité),
  « intégrée » = ajoutée à l'espace d'états avant le plat (plat+requête).

## Limites (rappel)

Proxys académiques, pas les instances MCC réelles ; la validation 328/328 est
une vérification sur énumération complète, pas une preuve formelle ; la
rentabilité des rungs reste l'affaire de l'oracle C4 et du jalon M7
(confrontation lisage-bench, témoins OxiDD/ADIAR).

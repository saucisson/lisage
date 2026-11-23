---
name: "etude-detecter-avant-reduire"
title: "Étude « surcharge » VII : détecter avant de réduire"
type: "text/markdown"
---

# Étude « surcharge » VII : détecter avant de réduire

Septième volet. La partie VI a établi la cinquième loi (la portée des gardes dans l'ordre des variables décide de tout) et laissé un chantier : transformer cette loi en décision de contrôleur. Cette partie construit et valide le détecteur : à partir des seules **lectures statiques des gardes**, sans énumérer le langage, retrouver les verdicts manuels des parties I à VI et prédire le plat exactement.

## 1. Trois lectures statiques

Un modèle y est vu comme : un ordre de variables, des cellules, et pour chaque cellule q une garde qui lit un ensemble R(q) symétrique (le support). Trois quantités :

1. **Rayon modèle ρ** : distance Chebyshev maximale entre la cellule écrite et ses lectures. Borné : la garde est évaluable dans une fenêtre géométrique. Croissant avec la taille : fenêtre interdite.
2. **Forme de requête** : pli de comptage avec seuil sur une classe de cellules (référendum : la majorité) : requête agrégative, ni la fenêtre ni la position ne la portent.
3. **Mémoire de coupe I(p)** : à chaque coupe p de l'ordre, l'ensemble des cellules futures directement interdites par les cellules occupées du préfixe, I = {v ≥ p : ∃u < p occupé, v ∈ R(u)}. Le nombre de motifs I distincts est la quantité que le plat doit réellement retenir à cette coupe.

La mémoire de coupe est la définition opérationnelle du **registre** de la partie VI : ce que l'empreinte d'occupation (lignes, colonnes, diagonales) encode pour les N-reines, ce que la bande d'épaisseur 1 encode pour les grilles, rien pour le référendum.

## 2. Le théorème du banc, et sa validation

**Affirmation** : le nombre de nœuds du plat MDD au niveau p égale exactement le nombre de motifs I(p) distincts parmi les préfixes réels. Chaque nœud du plat est un motif d'interdits résiduels.

Validation sur cinq familles, 328 niveaux au total, **égalité parfaite à 100 %** :

| Famille | n testés | niveaux | égalité nodes = \|I(p)\| |
|---|---|---|---|
| anneau C_n | 3 à 9 | 42 | 42/42 |
| grille plan n×n | 2 à 5 | 54 | 54/54 |
| grille tore n×n | 2 à 5 | 54 | 54/54 |
| N-reines n×n | 4 à 7 | 126 | 126/126 |
| référendum N | 3 à 10 | 52 | 52/52 |

Contre-vérifications croisées : les plats totaux retrouvent exactement les mesures des parties précédentes (plan 5 : 249, tore 5 : 1 159, N-reines 7 : 15 180).

Au passage, le banc réfute une première hypothèse de travail, formulée au lancement : les nœuds ne sont pas les projections des préfixes sur les cellules encore lues (égalité seulement 36 à 56 % des niveaux). Le plat ne retient pas les valeurs, seulement les interdits qu'elles imposent : la projection survit, la mémoire de coupe est la bonne définition.

Conséquence centrale : **le plat moderne est la table de hachage des motifs I**. Analyser les gardes, c'est prédire le plat, niveau par niveau, sans construire l'espace d'états. Et toute réduction annotée (fenêtre, compteur, registre généralisé) est une compression de cette table : elle ne peut prétendre qu'à regrouper les motifs équivalents.

## 3. Les familles sous le détecteur

**Anneau C_n** (partie I) : la mémoire de coupe plafonne à 4 motifs dès n=6, une bande fine constante : le plateau de la partie I, retrouvé par la seule lecture des supports. |S| suit les nombres de Lucas, la mémoire ne suit pas.

**Grilles** (partie V) : mémoire de coupe aux fins de lignes, n=5 :

- plan : [8, 13, 13, 13, 2] : bande Fibonacci constante d'une ligne à l'autre ;
- tore : [8, 61, 112, 50, 2] : même démarrage, puis **bourrelet**.

La couture de la partie V est visible statiquement : c'est l'excès de mémoire de coupe imposé par l'enroulement (les gardes de la dernière ligne relisent la première). Le détecteur produit la loi partie V (la couture porte le gain) sans énumérer le langage : le gain possible est proportionnel à la taille de la table que la couture gonfle.

**Référendum** (partie VI-A) : mémoire de coupe vide à toutes les coupes (garde libre), plat = N exactement, un nœud par niveau. Le détecteur ne voit rien à réduire dans la dynamique ; la requête majoritaire seule porte le drapeau agrégatif : verdict compteur, conformément à la partie VI (les types sans multiplicités y étaient incorrects dès N=7).

**N-reines** (partie VI-B) : la mémoire de coupe explose : 17, 59, 265, 1 146 motifs pour N=4 à 7 (×3,5 à 4,5 par pas), rayon modèle N-1. Verdict registre : la fenêtre bornée est déclarée incorrecte avant toute construction, conforme au taux d'erreur mesuré en partie VI (20 à 40 % de faux positifs).

## 4. Le tableau des verdicts

Le classifieur lit (rayon, requête, profil de coupe) et tranche : requête agrégative → compteur ; rayon borné et mémoire en bande fine → fenêtre ; rayon croissant et mémoire croissante → registre.

| Famille | ρ | Requête | Mémoire de coupe | Verdict détecté | Verdict historique | Accord |
|---|---|---|---|---|---|---|
| anneau C_n | 1 | aucune | plafonnée (4) | fenêtre | I : la fenêtre/handles porte le gain | oui |
| grille plan | 1 | aucune | bande fine (13) | fenêtre | V : fenêtre correcte, surcharge non rentable | oui |
| grille tore | 1 | aucune | bande + bourrelet | fenêtre + couture | V : la couture porte le gain | oui |
| référendum | 0 (garde libre) | comptage-seuil | vide | compteur | VI-A : multiplicités/compteur | oui |
| N-reines | n-1 | aucune | croissance ×4/pas | registre | VI-B : fenêtre fausse, registre | oui |

Cinq accords sur cinq. Le détecteur ne dit pas si la surcharge est rentable (le plat du plan reste imbattable à petite taille tout en autorisant une fenêtre correcte) : il garantit la correction des rungs disponibles, la rentabilité restant l'affaire de l'oracle C4 et du jalon M7. Une réduction inexacte reste disqualifiée d'office : le détecteur est le filtre qui l'élimine avant construction.

## 5. Doctrine pour lisage-lang

Le contrôleur peut enchaîner, tout statiquement :

1. collecter les supports de lecture des gardes et requêtes (passe sur l'AST des gardes) ;
2. calculer rayon modèle et drapeaux agrégatifs ;
3. énumérer (ou échantillonner) les motifs I le long de l'ordre canonique : une passe linéaire dans la construction du plat, ou une simulation à budget borné avant toute décision ;
4. choisir le rung selon le tableau des verdicts, puis seulement juger la rentabilité entre options correctes.

La partie VI répondait « quand la garde est globale, la fenêtre est fausse » ; la partie VII répond « et le contrôleur peut le savoir avant de réduire ». Le triangle (rayon, agrégat, mémoire de coupe) est le formulaire de déclaration des gardes.

## 6. Limites

- Mémoire de coupe validée sur énumération complète (petites instances) ; dans lisage, elle se calcule par la construction du plat lui-même ou par échantillonnage, la validation 328/328 ne couvre pas encore ce mode dégradé ;
- ordre des variables fixé (naturel, ligne-par-ligne) : la partie VI a montré que l'ordonnancement ne sauve pas les N-reines (~6 %), mais l'interaction détecteur-ordonnancement reste à explorer ;
- la détection du pli de comptage est déclarée dans le banc, pas démontrée sur des gardes syntaxiques : le pattern-matching AST (Σ avec seuil sur une classe) reste à implémenter dans lisage-lang ;
- proxys académiques : familles MCC réelles (le FMS de la partie III, le Referendum du catalogue) à confronter via lisage-bench, jalon M7 ;
- la mémoire de coupe dit la taille du plat, pas le coût d'évaluation des transitions (le gain du compteur EV-MDD, cf. partie VI).

Données de la partie VII conservées avec le projet (results7.json : cinq familles, niveaux, mémoires de coupe et profils de couture ; banc complet en moins de dix secondes).
# Liseuse : voir Lisage travailler

**Liseuse** est l'outil de visualisation du projet : elle montre un **ordre** (et son contrôleur), un **diagramme de décision** (le pool, ses handles, ses réductions), et **l'application d'une opération** sur ce diagramme, pas à pas. Le nom suit la règle de Lisage : la liseuse est à la fois celle qui lit et la lectrice (lire + Lisage).

## 1. Rôle et non-rôles

**Rôle** : rendre visibles les trois affirmations du projet que le texte ne peut que constater : la canonicité par handle (C1 : deux formes identiques sous deux ancres différentes ne fusionnent pas), le travail de saturation (rounds, firings, mémoire de niveau), et le partage (multiplicités, quotient O(1) + bord). Un œil sur le pool vaut mille affichages de débogage.

**Non-rôles assumés** : ce n'est pas un débogueur de code Rust (pas de points d'arrêt, pas d'inspection mémoire du processus) ; ce n'est pas un éditeur de graphe (lecture seule, jamais d'édition du diagramme) ; ce n'est pas un serveur (fichiers autonomes, consultables hors ligne, committables dans le dépôt).

**Contrainte de fond** : le déterminisme. Une trace rejouée doit être identique à la trace d'origine (l'exigence même du jalon 1 : interprète de référence + property-based). Liseuse ne fait donc que **rejouer** des événements émis par le moteur ; elle n'interroge jamais le moteur en direct.

## 2. Architecture : event-sourced, trois briques, un fichier

```
┌─────────────── lisage-core ───────────────┐
│  port d'instrumentation : trait Observer  │
│  (no-op par défaut, zéro coût en release) │
└──────────────┬───────────────────────────┘
               │ événements typés
               ▼
┌────────────── lisage-liseuse ─────────────────────────────┐
│  1. trace  : sérialiseur JSONL versionné (schema rédigé)   │
│  2. index : reconstruction des états (replay déterministe) │
│             + fenêtres (par round, par opération, par pool)│
│  3. rendu : SVG du diagramme, HTML autonome, export figures│
└──────────────┬────────────────────────────────────────────┘
               │ un fichier unique, ouvrable hors ligne
               ▼
        liseuse.html  (SVG + scrubber JS minimal, aucun serveur,
                       aucune dépendance réseau, committable)
```

Les trois briques sont des modules de la crate `lisage-liseuse`, une crate de plus du workspace (voir la feuille de route, M0-2). La sortie est un **fichier HTML autonome** (le SVG et le scrubber y sont embarqués) : on le met dans `bench-results/`, on le commit, on l'envoie par courriel, on l'ouvre dans une revue.

## 3. Le format de trace (le contrat entre le moteur et l'œil)

JSONL, une ligne = un événement, schéma versionné (`{"v":1, ...}`). Types d'événements :

| Événement | Champs | Ce que ça rend visible |
|---|---|---|
| `HandleCreated` | contrôleur, ancre, forme, `NodeId` | la naissance d'un handle : la clé (contrôleur, ancre, forme) qui fonde C1 |
| `Reduction` | type (R-share, R-collapse, R-default, D-abs), ids | les quotients quand ils tirent |
| `Memo` | op, arguments, hit/miss | le prix du pli (L3) |
| `Attr` | canal (hérité, synthétisé, mémoire), nœud, valeur | les trois flux, la pièce centrale de Lisage |
| `Fire` | classe d'événement, nœud, round | la saturation au travail |
| `Round` | début/fin, niveau, compteur de passes | les rounds et leur convergence |
| `Convert` | contrôleur source, cible, coût | les royaumes et les conversions à coût prédictible |
| `Pool` | plan, créations, vivants, GC | le bi-plan, la mémoire |
| `Snapshot` | (throttle) état complet du pool | les points d'ancrage du scrubber sur les longues traces |

Deux règles de conception : (i) le port `Observer` est **le seul** point de sortie (le pool n'inspecte jamais ⊕, et l'instrumentation non plus : elle observe des identifiants et des compteurs, pas la sémantique) ; (ii) le volume est **borné par fenêtrage** : une trace complète par round, une trace grossière (compteurs + snapshots) pour les longues exécutions ; le réglage est un simple entier (événements par fenêtre).

## 4. Vue 1 : l'ordre et les contrôleurs

Ce que la vue montre : **O**, la décomposition et les conversions, c'est-à-dire les paramètres du contrôleur étendu `C = (δ, Σ, décomposition + O, R, défauts, D^déf, encodage, attribution)`.

- **La fiche contrôleur** : les huit champs de `C` en un panneau, avec leurs valeurs pour l'instance courante. C'est la légende de tout le reste : chaque couleur, chaque forme de nœud, chaque arc pointillé affiché ensuite s'y rattache.
- **L'ordre comme piste** : O linéaire en une piste horizontale (les niveaux dans l'ordre de balayage) ; O structuré en arbre dessiné comme un arbre (le vtree des SDD sententiels s'y installera sans cas particulier).
- **Les royaumes côte à côte** : quand deux contrôleurs coexistent (le témoin BDD classique et l'instance de référence, par exemple), deux pistes superposées, et la conversion fléchée avec son coût mesuré (événements `Convert`).
- **Le Σ déplié** : à côté de la piste, l'expression de Σ qui déplie les niveaux (le record des philosophes, la variable répétée avec son marqueur de fin pour `Seq`/`Set`/`Map`), pour voir que l'ordre est un dépliage, pas une liste de variables.

## 5. Vue 2 : le diagramme

Le pool dessiné comme un DAG à niveaux : une bande horizontale par clé de Σ (le layout est donné par l'ordre, pas calculé : nos diagrammes sont ordonnés par construction, aucun Sugiyama nécessaire).

- **Les nœuds** : rectangles arrondis = handles ; survol = le triple (contrôleur, ancre, forme) et la multiplicité (combien de réplicas positionnels ce handle représente) ; un badge ×k quand k > 1.
- **Les arcs** : étiquetés par les clés ; les arcs implicites de `D^déf` en pointillés gris (l'absence dessinée comme absence) ; le marqueur de fin de `list/set/map` comme un petit disque plein.
- **Les ancres** : la valuation d'arête (shift, poids, complément) en couleur d'arc, avec la légende de la fiche contrôleur.
- **Les clés-régions** (δ continu) : quand les clés sont des régions (intervalles, contraintes DBM, singletons dégénérés), l'arc porte le **terme** de la clé tel quel ; Liseuse l'affiche sans l'interpréter (le pool ne l'interprète jamais), et la fiche contrôleur documente l'algèbre de clés qui fonde sa lecture.
- **Le zoom sémantique** : dézoomer groupe par classes du quotient (formes × ancres : les 5 signatures des philosophes plutôt que les 15 nœuds vivants à n = 5) ; zoomer revient aux réplicas positionnels. Le quotient est visible d'un mouvement de molette : c'est la contribution n° 8 rendue palpable.
- **Le pool en fond** : les nœuds groupés par plan (principal/ombre), les collectes du GC en fondu.

```
Exemple (philosophes, vue quotient, 5 signatures) :

  record : A(3)  B(3)  ...           Σ déplié : Philo(n) = record[Goût; ×(Fourchette, Fourchette)]
  ────────────────────────────────  piste O : g₁ f₁ g₂ f₂ g₃ f₃ (intercalage du record)
   S ──▶ ⬤/⊤ ──▶ A₀ ──▶ A₁
            ╲────▶ B₀ ──▶ B₁        handles : S, A₀, A₁, B₀, B₁, ⊤, ⊥ (7 avec les feuilles)
  ────────────────────────────────
   ×1     ×6     ×5     ×2 ...      multiplicités : le plat paie n, Lisage paie une forme
```

## 6. Vue 3 : l'opération, pas à pas

C'est la vue qui n'existe nulle part dans les paquets existants : **le déroulé d'une transduction attribuée**, rejoué événement par événement sur le diagramme.

- **Le scrubber** : une frise des événements de la fenêtre (un tir, un round, une opération complète) ; on scrubbe, le diagramme montre l'état à cet instant. Les `Snapshot` bornent la reconstruction ; entre deux, le replay incrémental.
- **Les trois canaux en couleur** : l'attribut **hérité** descend en bleu (l'arc porte une valeur vers le bas), le **synthétisé** remonte en orange (`φ_aggr` agrège les fils), la **mémoire de niveau** pulse en violet quand elle écrit. `lookup` devient lisible : la clé descend, le fragment remonte.
- **Les mémos** : halo vert (hit) ou rouge (miss) sur le nœud visité ; le compteur hit/miss en tableau de bord, le prix du pli en direct.
- **Les réductions** : un flash au `Reduction` (deux nœuds fusionnent en gardant le survivant ; les fantômes des nœuds absorbés restent un instant en filigrane).
- **Les rounds de saturation** : barres verticales sur la frise (round k = tirer toutes les classes d'événements locales à un niveau avant de remonter) ; compteur de passes, firings, vivants, pic : les grandeurs mesurées dans l'exemple des philosophes, en direct.
- **Le tableau de bord** : created, live, peak, passes, firings, memo hit-rate : exactement les colonnes des tables de mesures, pour qu'une figure et un tableau de bench parlent le même langage.

## 7. Cas d'usage étalons

1. **Philosophes n = 3, 4, 5** : un tir complet en pas à pas (vue 3), puis la saturation rounds par rounds, puis la vue quotient ; la figure finale (7 formes, O(1) + bord) est le premier livrable de l'article. `fold` (le comptage) rejoué : le pli remonte, la table montre le cardinal exact 152 / 23 168 / 536 754 176.
2. **Le gaucher** : même exemple, un philosophe inversé ; la vue montre la racine s'enrichir (7 clés) et l'intérieur **inchangé** : les formes intérieures restent partagées. C'est la promesse du bord, dessinée.
3. **C1 en action** : deux nœuds de même forme sous deux ancres différentes restent deux handles ; la vue les affiche côte à côte avec leur triple. Le PBT vérifie, Liseuse le montre.
4. **Le témoin BDD** : la même fonction sous le contrôleur BDD classique, conversion fléchée entre les royaumes, coût mesuré.
5. **Figures d'article** : mode export (M5-5) : SVG propre, thème serif, légende auto-générée depuis la fiche contrôleur ; le même fichier qui sert au débogage sert à la publication.

## 8. Limites et passages à l'échelle

- **Taille de trace** : le fenêtrage borne tout ; en mode grossier (compteurs + snapshots), une exécution MCC complète tient en quelques Mo de JSONL. Le réglage : fenêtre par round, ou par opération, ou fixe.
- **Taille de diagramme** : la vue cible les cas pédagogiques et de bench (jusqu'à ~10 000 nœuds rendus, au-delà le zoom sémantique passe au quotient et aux compteurs). Liseuse n'est pas un profileur de très grands espaces : c'est l'œil du protocole, pas la loupe du datacenter.
- **Aucun serveur, aucune dépendance JS** : un fichier HTML, du SVG, quelques centaines de lignes de JS vanilla pour le scrubber. Si le JS devient un fardeau, la sortie SVG statique reste complète (le scrubber devient une suite de fichiers numérotés).

## 9. Découpage (rattaché aux jalons de la feuille de route)

| Livrable | Jalon | Contenu |
|---|---|---|
| Port `Observer` + schéma de trace v1 | M1-9 | les événements du tableau § 3, le sérialiseur, le replay |
| Vue diagramme statique (SVG) | M1-9 | niveaux, handles, ancres, arcs implicites |
| Trace pas à pas + canaux + mémos | M2-6 | le scrubber, les trois canaux, les réductions, les rounds |
| Vue ordre/contrôleurs/conversions | M3-4 | fiche contrôleur, pistes, royaumes, Σ déplié |
| Vues mémoire et quotient | M4-5 | pools, GC, badges de multiplicité, zoom sémantique |
| Export figures articles | M5-5 | thème publication, légendes auto |

**Critère d'acceptation transversal** : à chaque jalon, le protocole de mesure correspondant (les tables de l'exemple des philosophes pour M4, la comparaison A/B/C pour M5) doit être reproductible **une figure Liseuse à l'appui**, générée depuis une trace archivée dans le dépôt.
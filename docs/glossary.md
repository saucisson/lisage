# FR → EN terminology glossary (M6-1)

Reference mapping for milestone M6 (English-only documentation and
code, issue #104). Every translation issue M6-2 … M6-7 and every code
rename in M6-5/M6-6 must use the terms below, so the whole repository
speaks one vocabulary.

## Domain terms

| French | English | Notes |
| --- | --- | --- |
| diagramme de décision | decision diagram | the core data structure |
| transduction attribuée | attributed transduction | |
| fenêtre glissante | sliding window | |
| fenêtre (absolue / relative) | window (absolute / relative) | |
| forme (relative / absolue / dépliée) | form (relative / absolute / unfolded) | |
| taille dépliée | unfolded size | |
| jalon | milestone | `jalon/M6` GitHub labels keep their scheme (labels, not prose) |
| atelier didactique | didactic workshop | the teaching playground (`atelier.rs`) |
| pool bi-plan | two-plane pool | the shared node store (`biplan.rs`) |
| témoin BDD | BDD witness | external reference implementation |
| schéma de référence | reference schema | |
| schéma (de variables) | (variable) schema | |
| liseuse | reader | the visualizer crate/tool |
| arc (d'entrée / de sortie) | (input / output) edge | |
| nœud | node | |
| racine | root | |
| feuille | leaf | |
| ordre (de variables) | (variable) order | |
| voie | lane | |
| arène | arena | memory region (`arena.rs`) |
| pli | fold | the `(ℕ,+,×)` fold operation |
| ancre (composée) | (composed) anchor | |
| fenêtre ancrée | anchored window | |
| état étendu | extended state | |
| fil rouge | golden thread | the worked example thread |
| table de vérité | truth table | |
| cardinal / cardinalité | cardinality | |
| domaine | domain | |
| attribut | attribute | |
| étiquette | label | |
| observateur | observer | |
| erreur (typée) | (typed) error | |
| pool | pool | the shared node store |
| algèbre de clés | key algebra | the `KeyAlgebra` contract |
| contrôleur | controller | the `Controller` contract |
| handle | handle | opaque reference to a diagram (`Handle`) |
| identifiant de nœud | node id | `NodeId` |
| mesure / métriques | measurement / metrics | |
| borne (inf / sup) | (lower / upper) bound | |
| seuil | threshold | |
| compteur | counter | |
| variable | variable | |
| ensemble | set | |
| clé | key | |
| valeur | value | |
| table / tableau | table | |
| chemin | path | |
| niveau | level | |
| taille | size | |
| hauteur / largeur / profondeur | height / width / depth | |
| couleur | color | |
| légende | legend | |
| vue | view | |
| région / zone | region / zone | |
| transition | transition | |
| réseau de Petri | Petri net | |
| anneau | ring | |
| produit / somme | product / sum | |
| quotient | quotient | |
| intervalle | interval | |
| borne | bound | |
| variable booléenne | Boolean variable | |
| littérale | literal | |
| clause | clause | |
| satisfaisable | satisfiable | |
| BDD / SDD / DDD / CDD / DBM | BDD / SDD / DDD / CDD / DBM | acronyms unchanged |
| CUDD | CUDD | external library name unchanged |
| confrontation | benchmark confrontation | `confrontation.rs` runner |
| différentiel | differential | `differentiel.rs` |
| triptyque | triptych | `triptyque.rs` |
| philosophes (N philosophes) | philosophers (dining philosophers) | the R_n worked example |
| charge (de travail) | workload | bench charges |
| aléatoire | random | |
| déterministe | deterministic | |
| graine | seed | |
| horizon | horizon | |
| lot | batch | |
| rapport | report | |
| trace | trace | |
| événement | event | |
| narration | narration | DEBUG logging narration (ADR-0005) |
| pile | stack | |
| file | queue | |
| sommet | apex / top | |
| parent / enfant / frère | parent / child / sibling | |
| branche | branch | |
| parcours | traversal | |
| visiteur / visiter | visitor / visit | |
| premier / dernier / suivant / précédent | first / last / next / previous | |
| trier / trié | sort / sorted | |
| filtrer / filtre | filter | |
| compiler / compilation | compile / compilation | |
| construire / construction | build / construction | |
| compter / compte | count | |
| mesurer | measure | |
| réduire / réduction | reduce / reduction | |
| saturer / saturation | saturate / saturation | |
| normaliser / normalisation | normalize / normalization | |
| canonique / canonicité | canonical / canonicity | |
| valider / valide / invalide | validate / valid / invalid | |
| comparer / comparaison | compare / comparison | |
| convertir / conversion | convert / conversion | |
| sérialiser / sérialisation | serialize / serialization | |
| lire / lecture | read | |
| écrire / écriture | write | |
| ouvrir / fermeture | open / closure | |
| extraire | extract | |
| disperser | scatter | |
| fusionner | merge | |
| couper | cut / split | |
| coller | paste | |
| empiler / dépiler | push / pop | |
| itérer / itération | iterate / iteration | |
| parcourir | iterate over | |
| tester / test | test | |
| épingler / épinglé | pin / pinned | test pins |
| rejeter / rejeté | reject / rejected | |
| valider | validate | |
| notifier | notify | the `notify` tool |
| tableau de bord | dashboard | |
| feuille de route | roadmap | |
| état de l'art | state of the art | |
| proposition | proposal | |
| analyse critique | critical analysis | |
| note de conception | design note | |
| décision (ADR) | decision record (ADR) | |
| protocole | protocol | |
| contrat | contract | |
| invariant | invariant | |
| précondition / postcondition | precondition / postcondition | |
| preuve | proof | |
| exemple | example | |
| échantillon | sample | |
| lot / morceau | chunk | |
| paquet | packet | |
| jeton | token | |
| balise | tag | |
| champ | field | |
| enregistrement | record | |
| type | type | |
| signature | signature | |
| interface | interface | |
| bibliothèque | library / crate | |
| couche | layer | |
| module | module | |
| paquet (crate) | crate | |
| espace de noms | namespace | |
| portée | scope | |
| durée de vie | lifetime | |
| emprunter | borrow | |
| posséder / propriétaire | own / owner | |
| référence | reference | |
| pointeur | pointer | |
| intelligent | smart | |
| brut | raw | |
| enveloppe | wrapper | |
| fabrique | factory | |
| constructeur | constructor | |
| destructeur | destructor | |
| itérateur | iterator | |
| closure / fermeture | closure | |
| fonction | function | |
| méthode | method | |
| procédure | procedure | |
| paramètre | parameter | |
| argument | argument | |
| retour | return | |
| résultat | result | |
| option | option | |
| erreur | error | |
| panique / panic | panic | |
| assertion | assertion | |
| commentaire | comment | |
| documentation (doc comment) | doc comment | |
| test | test | |
| banc d'essai | benchmark | |
| couverture | coverage | |
| lint / linter | lint / linter | |
| formateur | formatter | |
| vérificateur de types | type checker | |
| branche (git) | branch | |
| fusion (merge) | merge | |
| rebase | rebase | |
| commit | commit | |
| étiquette (git tag) | tag | |
| dépôt | repository | |
| distant | remote | |
| branche distante | remote branch | |
| pull request | pull request | |
| revue / review | review | |
| problème / issue | issue | |
| étiquette (label) | label | |
| jalon (milestone) | milestone | |
| propriétaire | owner | |
| responsable | maintainer | |
| contributeur | contributor | |
| humain | human | |
| agent | agent | |
| session | session | |
| boucle | loop | |
| tâche | task | |
| file d'attente | queue | |
| processus | process | |
| fil / thread | thread | |
| asynchrone | async | |
| attente | wait | |
| délai / timeout | timeout | |
| reprise | retry | |
| abandon | abort | |
| succès / échec | success / failure | |
| avertissement | warning | |
| information | info | |
| débogage / debug | debug | |
| niveau de log | log level | |
| secret | secret | |
| jeton (auth) | token | |
| clé (secrète) | secret key | |
| dépôt (registry) | registry | |
| image | image | |
| conteneur | container | |
| volume | volume | |
| réseau | network | |
| port | port | |
| hôte | host | |
| domaine (DNS) | domain | |
| URL | URL | unchanged |
| chemin (fichier) | path | |
| fichier | file | |
| répertoire | directory | |
| lien | link | |
| lien symbolique | symlink | |
| permission | permission | |
| rôle | role | |
| politique | policy | |
| règle | rule | |
| quota | quota | |
| budget | budget | |
| plafond | ceiling | |
| plancher | floor | |
| moyenne | average | |
| médiane | median | |
| écart-type | standard deviation | |
| ratio | ratio | |
| taux | rate | |
| pourcentage | percentage | |
| durée | duration | |
| délai | delay | |
| heure | time / hour | |
| date | date | |
| fuseau horaire | timezone | |
| calendrier | calendar | |
| planification | schedule | |
| tâche planifiée | cron job | |
| déclencheur | trigger | |
| crochet | hook | |
| extension | extension | |
| greffon | plugin | |
| connecteur | connector | |
| serveur | server | |
| client | client | |
| requête | request | |
| réponse | response | |
| en-tête | header | |
| corps | body | |
| charge utile | payload | |
| paramètre de requête | query parameter | |
| code de statut | status code | |
| paginer | paginate | |
| curseur | cursor | |
| jeton (pagination) | token | |
| limite | limit | |
| décalage | offset | |
| tri | sort | |
| filtre | filter | |
| recherche | search | |
| trier par | order by | |
| grouper par | group by | |
| agréger | aggregate | |
| jointure | join | |
| table (BDD) | table | |
| index | index | |
| cache | cache | |
| tampon | buffer | |
| flux | stream | |
| canal | channel | |
| verrou | lock | |
| mutex | mutex | |
| sémaphore | semaphore | |
| barrière | barrier | |
| condition | condition variable | |
| atomique | atomic | |
| ordonnancement | scheduling | |
| priorité | priority | |
| file de priorité | priority queue | |
| tas | heap | |
| arbre | tree | |
| graphe | graph | |
| graphe acyclique dirigé | DAG | |
| liste chaînée | linked list | |
| table de hachage | hash table | |
| ensemble (hash) | hash set | |
| file double | deque | |
| bit | bit | |
| octet | byte | |
| entier | integer | |
| flottant | float | |
| booléen | Boolean | |
| caractère | character | |
| chaîne | string | |
| tableau (array) | array | |
| vecteur | vector | |
| matrice | matrix | |
| dimension | dimension | |
| scalaire | scalar | |
| tenseur | tensor | |
| algèbre | algebra | |
| corps | field | |
| groupe | group | |
| monoïde | monoid | |
| catégorie | category | |
| foncteur | functor | |
| morphisme | morphism | |
| isomorphisme | isomorphism | |
| endomorphisme | endomorphism | |
| homomorphisme | homomorphism | |
| noyau | kernel | |
| rang | rank | |
| déterminant | determinant | |
| valeur propre | eigenvalue | |
| vecteur propre | eigenvector | |
| base | basis | |
| générateur | generator | |
| relation | relation | |
| équivalence | equivalence | |
| ordre partiel | partial order | |
| ordre total | total order | |
| treillis | lattice | |
| borne inférieure / supérieure | infimum / supremum | |
| plus petit / grand élément | least / greatest element | |
| chaîne | chain | |
| antichaîne | antichain | |
| élément maximal / minimal | maximal / minimal element | |
| borne supérieure (lub) | join | |
| borne inférieure (glb) | meet | |
| complément | complement | |
| différence | difference | |
| différence symétrique | symmetric difference | |
| intersection | intersection | |
| union | union | |
| produit cartésien | Cartesian product | |
| sous-ensemble | subset | |
| sur-ensemble | superset | |
| ensemble vide | empty set | |
| singleton | singleton | |
| partition | partition | |
| classe d'équivalence | equivalence class | |
| fonction | function | |
| application | map / apply | |
| antécédent | preimage | |
| bijection | bijection | |
| injection | injection | |
| surjection | surjection | |
| identité | identity | |
| composition | composition | |
| inverse | inverse | |
| associativité | associativity | |
| commutativité | commutativity | |
| distributivité | distributivity | |
| idempotence | idempotence | |
| élément neutre | identity element | |
| absorbant | absorbing element | |
| simplifiable | cancellative | |
| régulier | regular | |
| inversible | invertible | |
| unitaire | unital | |
| anneau commutatif | commutative ring | |
| corps commutatif | field | |
| anneau intègre | integral domain | |
| idéal | ideal | |
| idéal premier | prime ideal | |
| idéal maximal | maximal ideal | |
| anneau principal | principal ideal ring | |
| anneau euclidien | Euclidean ring | |
| PGCD / PPCM | GCD / LCM | |
| nombre premier | prime number | |
| factorisation | factorization | |
| divisibilité | divisibility | |
| congruence | congruence | |
| modulo | modulo | |
| reste | remainder | |
| quotient (division) | quotient | |
| valeur absolue | absolute value | |
| signe | sign | |
| positif / négatif | positive / negative | |
| nul / zéro | zero | |
| un / deux / trois | one / two / three | |
| pair / impair | even / odd | |
| maximum / minimum | maximum / minimum | |
| borne | bound | |
| plafond (math) | ceiling | |
| plancher (math) | floor | |
| arrondi | rounding | |
| troncature | truncation | |
| précision | precision | |
| erreur d'arrondi | rounding error | |
| virgule flottante | floating point | |
| exposant | exponent | |
| mantisse | mantissa | |
| norme | norm | |
| produit scalaire | dot product | |
| produit vectoriel | cross product | |
| déterminant | determinant | |
| transposée | transpose | |
| inverse (matrice) | inverse | |
| rang | rank | |
| diagonale | diagonal | |
| symétrique | symmetric | |
| définie positive | positive definite | |

## Naming rules for code and files

- Function and variable names: `snake_case` English (`construire_fenetre` → `build_window`).
- Type, trait and module names: `UpperCamelCase` / `snake_case` English (`ErreurConfrontation` → `ConfrontationError`, `temoin_bdd.rs` → `bdd_witness.rs`).
- Constants: `UPPER_SNAKE_CASE` English.
- File names: English, kebab-case for Markdown (`feuille-de-route-lisage-jalons-et-issues.md` → `roadmap-lisage-milestones-and-issues.md`), snake_case for Rust modules.
- Acronyms stay uppercase when they are proper names (BDD, SDD, CUDD, DAG, GCD); otherwise they follow the case of the surrounding name (`bdd_witness`, not `BddWitness`).
- Test names: English, descriptive (`test_fenetre_glissante` → `test_sliding_window`).
- Log/DEBUG event names and fields: English (ADR-0005 narration).
- User-facing strings (errors, CLI output, panics): English.
- GitHub labels keep their existing scheme (`jalon/M6` stays, it is a label not prose); prose in issues/PRs moves to English.

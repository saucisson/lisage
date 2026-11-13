# Confrontation lisage / témoins

Répétitions par cas : 5, processus frais à chaque mesure (protocole : docs/bench-confrontation-temoins-et-mesures.md).

## alea-lcg

| outil | variante | paramètres | encodage | temps médian (ms) | temps min (ms) | temps max (ms) | RSS médiane (Ko) | nœuds | vérifié |
|---|---|---|---|---|---|---|---|---|---|
| lisage | bdd | n=12 | - | 2.271 | 2.225 | 2.618 | 2680 | 727 | 2074 |
| cudd | bdd | n=12 | - | 0.383 | 0.373 | 0.409 | 14484 | 637 | 2074 |
| lisage | bdd | n=16 | - | 36.174 | 36.079 | 37.159 | 7020 | 8313 | 32698 |
| cudd | bdd | n=16 | - | 5.622 | 5.130 | 6.073 | 15352 | 8058 | 32698 |
| lisage | bdd | n=20 | - | 574.903 | 571.756 | 589.797 | 56212 | 106760 | 523228 |
| cudd | bdd | n=20 | - | 114.103 | 102.966 | 115.706 | 23128 | 93949 | 523228 |
| lisage | bdd | n=24 | - | 9954.819 | 9627.771 | 10537.391 | 639312 | 1071698 | 8386976 |
| cudd | bdd | n=24 | - | 2079.962 | 2025.067 | 2244.810 | 141984 | 1081280 | 8386976 |
| lisage | bdd | n=4 | - | 0.041 | 0.039 | 0.053 | 2284 | 9 | 7 |
| cudd | bdd | n=4 | - | 0.167 | 0.066 | 0.711 | 14436 | 8 | 7 |
| lisage | bdd | n=8 | - | 0.185 | 0.182 | 0.215 | 2320 | 74 | 137 |
| cudd | bdd | n=8 | - | 0.101 | 0.089 | 0.210 | 14548 | 69 | 137 |

## philosophes

| outil | variante | paramètres | encodage | temps médian (ms) | temps min (ms) | temps max (ms) | RSS médiane (Ko) | nœuds | vérifié |
|---|---|---|---|---|---|---|---|---|---|
| lisage | mdd | n=3 | - | 0.427 | 0.383 | 0.489 | 2416 | 33 | 20 |
| cudd | bdd | n=3 | - | 0.179 | 0.159 | 0.191 | 14736 | 29 | 20 |
| lisage | mdd | n=4 | - | 1.100 | 1.079 | 1.193 | 2628 | 53 | 56 |
| cudd | bdd | n=4 | - | 0.293 | 0.260 | 0.412 | 14732 | 49 | 56 |
| lisage | mdd | n=5 | - | 3.404 | 3.358 | 3.574 | 3308 | 73 | 152 |
| cudd | bdd | n=5 | - | 0.577 | 0.556 | 0.841 | 14816 | 69 | 152 |
| lisage | mdd | n=6 | - | 10.826 | 10.567 | 14.218 | 4800 | 93 | 416 |
| cudd | bdd | n=6 | - | 1.521 | 1.458 | 1.644 | 14968 | 89 | 416 |
| lisage | mdd | n=7 | - | 39.793 | 39.338 | 42.398 | 10416 | 113 | 1136 |
| cudd | bdd | n=7 | - | 5.187 | 4.876 | 5.606 | 15200 | 109 | 1136 |

## fenetre

| outil | variante | paramètres | encodage | temps médian (ms) | temps min (ms) | temps max (ms) | RSS médiane (Ko) | nœuds | vérifié |
|---|---|---|---|---|---|---|---|---|---|
| lisage | mdd | nb_bases=10, taille=16 | absolu | 0.055 | 0.052 | 0.056 | 2284 | 12 | 160 |
| lisage | mdd | nb_bases=10, taille=16 | relatif | 0.027 | 0.025 | 0.029 | 2272 | 3 | 160 |
| cudd | bdd | nb_bases=10, taille=16 | - | 0.251 | 0.229 | 0.276 | 14484 | 28 | 160 |
| lisage | mdd | nb_bases=10, taille=4 | absolu | 0.037 | 0.036 | 0.038 | 2260 | 12 | 40 |
| lisage | mdd | nb_bases=10, taille=4 | relatif | 0.025 | 0.021 | 0.028 | 2280 | 3 | 40 |
| cudd | bdd | nb_bases=10, taille=4 | - | 0.105 | 0.096 | 0.155 | 14484 | 29 | 40 |
| lisage | mdd | nb_bases=10, taille=8 | absolu | 0.038 | 0.037 | 0.043 | 2300 | 12 | 80 |
| lisage | mdd | nb_bases=10, taille=8 | relatif | 0.025 | 0.025 | 0.026 | 2300 | 3 | 80 |
| cudd | bdd | nb_bases=10, taille=8 | - | 0.148 | 0.142 | 0.250 | 14580 | 30 | 80 |

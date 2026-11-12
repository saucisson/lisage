/*
 * Binaire de confrontation CUDD (ADR-0102, décision humaine du 10 nov.
 * 2026 : témoins externes en conteneurs Docker locaux, hors CI).
 *
 * Une charge, une mesure, une ligne JSON sur stdout. Codes de sortie :
 * 0 = mesure rendue, 1 = échec de charge ou de vérification, 2 = usage.
 *
 * Le protocole complet vit dans
 * docs/bench-confrontation-temoins-et-mesures.md. Bloc mesuré, identique
 * chez tous les outils : lecture de l'entrée + construction du diagramme
 * + comptage de vérification. L'initialisation du manager CUDD est hors
 * du bloc (elle n'est pas la charge comparée).
 *
 * Charges (les mêmes que le runner lisage `confrontation`) :
 *   alea-lcg <n>               table de vérité LCG déterministe (mêmes
 *                              constantes que la ligne de base M1-8),
 *                              BDD par expansion de Shannon ;
 *   philosophes <n> <chemin>   R_n du fil rouge lu depuis le fichier
 *                              d'états étendus partagé, BDD par union
 *                              de cubes (4n variables booléennes :
 *                              pc_i et left_i sur 2 bits chacune) ;
 *   fenetre <taille> <nb_bases>  relation fenêtre glissante
 *                              (base, position), un cube par paire
 *                              (b sur bits, x sur bits).
 *
 * Gestion CUDD : chaque résultat conservé est référencé (Cudd_Ref) puis
 * libéré (Cudd_RecursiveDeref) ; les constantes complémentées ne sont
 * jamais référencées directement (Cudd_Ref exige un pointeur régulier).
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <sys/resource.h>
#include <cudd.h>

/* Graine du LCG (mêmes constantes que benches/micro.rs, M1-8). */
#define GRAINE_LCG 0x9E3779B97F4A7C15ULL
#define MULTIPLICATEUR_LCG 6364136223846793005ULL
#define INCREMENT_LCG 1442695040888963407ULL

/* Nombre maximal de variables de la charge alea-lcg (garde mémoire). */
#define VARIABLES_MAXIMALES_ALEA 24

/* Nombre maximal de philosophes (garde : 4n variables). */
#define PHILOSOPHES_MAXIMAUX 64

/* Codes de sortie : 1 = échec, 2 = usage. */
#define CODE_ECHEC 1
#define CODE_USAGE 2

/* Temps écoulé entre deux instants, en millisecondes. */
static double temps_ms(const struct timespec *debut, const struct timespec *fin) {
    return (double)(fin->tv_sec - debut->tv_sec) * 1000.0
         + (double)(fin->tv_nsec - debut->tv_nsec) / 1000000.0;
}

/* RSS maximale du processus en kilo-octets (ru_maxrss, Linux : Ko). */
static unsigned long rss_max_ko(void) {
    struct rusage usage;
    if (getrusage(RUSAGE_SELF, &usage) != 0) {
        return 0;
    }
    return (unsigned long)usage.ru_maxrss;
}

/* Écrit la ligne JSON de mesure (ordre des clés fixe, épinglé par le
 * protocole). Les chaînes viennent d'un vocabulaire contrôlé (noms
 * d'outils, de variantes, de charges, entiers) : aucun échappement
 * dynamique n'est nécessaire. */
static void ecrire_mesure(const char *outil,
                          const char *variante,
                          const char *charge,
                          const char *parametres,
                          double temps,
                          unsigned long rss,
                          unsigned long noeuds,
                          unsigned long verifie) {
    printf("{\"outil\":\"%s\",\"variante\":\"%s\",\"charge\":\"%s\",%s,"
           "\"temps_ms\":%.3f,\"rss_max_ko\":%lu,\"noeuds\":%lu,\"verifie\":%lu}\n",
           outil, variante, charge, parametres, temps, rss, noeuds, verifie);
    fflush(stdout);
}

/* Expansion de Shannon descendante : table[a] = valeur de la fonction
 * sur l'affectation a (bit ℓ de a = variable ℓ), comme cudd_count.c.
 * Le résultat est référencé : l'appelant en devient propriétaire. */
static DdNode *construire_depuis_table(DdManager *manager,
                                       DdNode **variables,
                                       int n,
                                       const unsigned char *table,
                                       int niveau) {
    if (niveau == n) {
        return table[0] ? Cudd_ReadOne(manager) : Cudd_Not(Cudd_ReadOne(manager));
    }
    int moitie = 1 << (n - niveau - 1);
    DdNode *bas = construire_depuis_table(manager, variables, n, table, niveau + 1);
    Cudd_Ref(bas);
    DdNode *haut = construire_depuis_table(manager, variables, n, table + moitie, niveau + 1);
    Cudd_Ref(haut);
    DdNode *ite = Cudd_bddIte(manager, variables[niveau], haut, bas);
    if (ite == NULL) {
        fprintf(stderr, "alea-lcg : ITE indisponible\n");
        exit(CODE_ECHEC);
    }
    Cudd_Ref(ite);
    Cudd_RecursiveDeref(manager, bas);
    Cudd_RecursiveDeref(manager, haut);
    return ite;
}

/* Charge alea-lcg : BDD de la table LCG, satisfactions comptées par
 * CUDD (vérification croisée : == poids de la table, falsifié). */
static int charge_alea_lcg(int n) {
    if (n < 1 || n > VARIABLES_MAXIMALES_ALEA) {
        fprintf(stderr, "alea-lcg : n doit être dans 1..%d\n", VARIABLES_MAXIMALES_ALEA);
        return CODE_ECHEC;
    }
    unsigned long taille = 1UL << n;
    unsigned char *table = (unsigned char *)malloc(taille);
    if (table == NULL) {
        fprintf(stderr, "alea-lcg : table non allouée\n");
        return CODE_ECHEC;
    }
    unsigned long etat = GRAINE_LCG;
    unsigned long poids = 0;
    for (unsigned long indice = 0; indice < taille; indice++) {
        etat = etat * MULTIPLICATEUR_LCG + INCREMENT_LCG;
        table[indice] = (unsigned char)(etat >> 63);
        poids += table[indice];
    }
    DdManager *manager = Cudd_Init(n, 0, CUDD_UNIQUE_SLOTS, CUDD_CACHE_SLOTS, 0);
    if (manager == NULL) {
        fprintf(stderr, "alea-lcg : manager CUDD indisponible\n");
        free(table);
        return CODE_ECHEC;
    }
    DdNode **variables = (DdNode **)malloc((size_t)n * sizeof(DdNode *));
    for (int i = 0; i < n; i++) {
        variables[i] = Cudd_bddNewVar(manager);
    }
    struct timespec debut, fin;
    clock_gettime(CLOCK_MONOTONIC, &debut);
    DdNode *fonction = construire_depuis_table(manager, variables, n, table, 0);
    unsigned long noeuds = (unsigned long)Cudd_DagSize(fonction);
    unsigned long satisfait = (unsigned long)Cudd_CountMinterm(manager, fonction, n);
    clock_gettime(CLOCK_MONOTONIC, &fin);
    Cudd_RecursiveDeref(manager, fonction);
    free(table);
    free(variables);
    Cudd_Quit(manager);
    if (satisfait != poids) {
        fprintf(stderr, "alea-lcg : vérification échouée (attendu %lu, obtenu %lu)\n",
                poids, satisfait);
        return CODE_ECHEC;
    }
    char parametres[32];
    snprintf(parametres, sizeof(parametres), "\"parametres\":{\"n\":\"%d\"}", n);
    ecrire_mesure("cudd", "bdd", "alea-lcg", parametres,
                  temps_ms(&debut, &fin), rss_max_ko(), noeuds, satisfait);
    return 0;
}

/* Charge philosophes : R_n lu depuis le fichier d'états étendus (première
 * ligne « n <n> », puis 2n valeurs par ligne : pc_0 left_0 pc_1 …).
 * Encodage booléen : pc_i sur 2 bits (variables 4i, 4i+1), left_i sur
 * 2 bits (variables 4i+2, 4i+3) ; valeur v = 0/1/2 → bits (v&1, v>>1).
 * Un cube par état, union par Cudd_bddOr ; les minterms sur 4n variables
 * doivent être le nombre d'états du fichier. La racine part vide (NULL) :
 * la constante complémentée ⊥ n'est jamais référencée directement. */
static int charge_philosophes(int n, const char *chemin) {
    if (n < 1 || n > PHILOSOPHES_MAXIMAUX) {
        fprintf(stderr, "philosophes : n doit être dans 1..%d\n", PHILOSOPHES_MAXIMAUX);
        return CODE_ECHEC;
    }
    int nombre_variables = 4 * n;
    DdManager *manager = Cudd_Init(nombre_variables, 0, CUDD_UNIQUE_SLOTS,
                                   CUDD_CACHE_SLOTS, 0);
    if (manager == NULL) {
        fprintf(stderr, "philosophes : manager CUDD indisponible\n");
        return CODE_ECHEC;
    }
    DdNode **variables = (DdNode **)malloc((size_t)nombre_variables * sizeof(DdNode *));
    for (int i = 0; i < nombre_variables; i++) {
        variables[i] = Cudd_bddNewVar(manager);
    }
    /* Bloc mesuré : lecture de l'entrée + construction + comptage (init du
     * manager hors bloc, comme chez lisage). */
    struct timespec debut, fin;
    clock_gettime(CLOCK_MONOTONIC, &debut);
    FILE *fichier = fopen(chemin, "r");
    if (fichier == NULL) {
        fprintf(stderr, "philosophes : fichier %s illisible\n", chemin);
        free(variables);
        Cudd_Quit(manager);
        return CODE_ECHEC;
    }
    int n_declare = 0;
    if (fscanf(fichier, "n %d", &n_declare) != 1 || n_declare != n) {
        fprintf(stderr, "philosophes : première ligne attendue « n %d »\n", n);
        fclose(fichier);
        free(variables);
        Cudd_Quit(manager);
        return CODE_ECHEC;
    }
    DdNode *racine = NULL;
    int *phase = (int *)malloc((size_t)nombre_variables * sizeof(int));
    unsigned long nb_etats_lus = 0;
    while (1) {
        int valeur = 0;
        int complet = 1;
        for (int i = 0; i < 2 * n; i++) {
            if (fscanf(fichier, "%d", &valeur) != 1) {
                complet = 0;
                break;
            }
            if (valeur < 0 || valeur > 2) {
                fprintf(stderr, "philosophes : valeur %d hors du domaine {0,1,2}\n", valeur);
                complet = 0;
                break;
            }
            /* Clé intercalée i : pc_(i/2) si i pair, left_(i/2) si i
             * impair ; chaque valeur sur 2 bits. */
            int philosophe = i / 2;
            int bit_bas = valeur & 1;
            int bit_haut = (valeur >> 1) & 1;
            if (i % 2 == 0) {
                phase[4 * philosophe] = bit_bas;
                phase[4 * philosophe + 1] = bit_haut;
            } else {
                phase[4 * philosophe + 2] = bit_bas;
                phase[4 * philosophe + 3] = bit_haut;
            }
        }
        if (!complet) {
            break;
        }
        DdNode *cube = Cudd_bddComputeCube(manager, variables, phase, nombre_variables);
        if (cube == NULL) {
            fprintf(stderr, "philosophes : cube indisponible\n");
            break;
        }
        Cudd_Ref(cube);
        if (racine == NULL) {
            racine = cube;
        } else {
            DdNode *suivant = Cudd_bddOr(manager, racine, cube);
            if (suivant == NULL) {
                fprintf(stderr, "philosophes : union indisponible\n");
                break;
            }
            Cudd_Ref(suivant);
            Cudd_RecursiveDeref(manager, cube);
            Cudd_RecursiveDeref(manager, racine);
            racine = suivant;
        }
        nb_etats_lus++;
    }
    if (racine == NULL) {
        fprintf(stderr, "philosophes : aucun état lu\n");
        free(phase);
        free(variables);
        fclose(fichier);
        Cudd_Quit(manager);
        return CODE_ECHEC;
    }
    unsigned long noeuds = (unsigned long)Cudd_DagSize(racine);
    unsigned long verifie = (unsigned long)Cudd_CountMinterm(manager, racine, nombre_variables);
    clock_gettime(CLOCK_MONOTONIC, &fin);
    Cudd_RecursiveDeref(manager, racine);
    free(phase);
    free(variables);
    fclose(fichier);
    Cudd_Quit(manager);
    if (verifie != nb_etats_lus) {
        fprintf(stderr, "philosophes : vérification échouée (attendu %lu, obtenu %lu)\n",
                nb_etats_lus, verifie);
        return CODE_ECHEC;
    }
    char parametres[32];
    snprintf(parametres, sizeof(parametres), "\"parametres\":{\"n\":\"%d\"}", n);
    ecrire_mesure("cudd", "bdd", "philosophes", parametres,
                  temps_ms(&debut, &fin), rss_max_ko(), noeuds, verifie);
    return 0;
}

/* Charge fenetre : la relation fenêtre glissante (base, position) :
 * paires (b, x) avec b dans [0, nb_bases) et x dans [b, b+taille).
 * Encodage booléen : b sur bits_base bits, x sur bits_position bits ;
 * un cube par paire, union par Cudd_bddOr ; les minterms sur
 * bits_base + bits_position variables doivent être nb_bases × taille. */
static int charge_fenetre(unsigned long taille, unsigned long nb_bases) {
    if (taille < 1 || nb_bases < 1) {
        fprintf(stderr, "fenetre : taille et nb_bases doivent être ≥ 1\n");
        return CODE_ECHEC;
    }
    int bits_base = 1;
    while ((1UL << bits_base) < nb_bases) {
        bits_base++;
    }
    unsigned long max_position = nb_bases + taille - 1;
    int bits_position = 1;
    while ((1UL << bits_position) < max_position) {
        bits_position++;
    }
    int nombre_variables = bits_base + bits_position;
    DdManager *manager = Cudd_Init(nombre_variables, 0, CUDD_UNIQUE_SLOTS,
                                   CUDD_CACHE_SLOTS, 0);
    if (manager == NULL) {
        fprintf(stderr, "fenetre : manager CUDD indisponible\n");
        return CODE_ECHEC;
    }
    DdNode **variables = (DdNode **)malloc((size_t)nombre_variables * sizeof(DdNode *));
    for (int i = 0; i < nombre_variables; i++) {
        variables[i] = Cudd_bddNewVar(manager);
    }
    unsigned long attendu = nb_bases * taille;
    int *phase = (int *)malloc((size_t)nombre_variables * sizeof(int));
    struct timespec debut, fin;
    clock_gettime(CLOCK_MONOTONIC, &debut);
    DdNode *racine = NULL;
    for (unsigned long base = 0; base < nb_bases; base++) {
        for (unsigned long position = base; position < base + taille; position++) {
            for (int bit = 0; bit < nombre_variables; bit++) {
                unsigned long valeur = (bit < bits_base) ? base : position;
                int bit_indice = (bit < bits_base) ? bit : bit - bits_base;
                phase[bit] = (int)((valeur >> bit_indice) & 1);
            }
            DdNode *cube = Cudd_bddComputeCube(manager, variables, phase, nombre_variables);
            if (cube == NULL) {
                fprintf(stderr, "fenetre : cube indisponible\n");
                break;
            }
            Cudd_Ref(cube);
            if (racine == NULL) {
                racine = cube;
            } else {
                DdNode *suivant = Cudd_bddOr(manager, racine, cube);
                if (suivant == NULL) {
                    fprintf(stderr, "fenetre : union indisponible\n");
                    break;
                }
                Cudd_Ref(suivant);
                Cudd_RecursiveDeref(manager, cube);
                Cudd_RecursiveDeref(manager, racine);
                racine = suivant;
            }
        }
    }
    if (racine == NULL) {
        fprintf(stderr, "fenetre : aucune paire construite\n");
        free(phase);
        free(variables);
        Cudd_Quit(manager);
        return CODE_ECHEC;
    }
    unsigned long noeuds = (unsigned long)Cudd_DagSize(racine);
    unsigned long verifie = (unsigned long)Cudd_CountMinterm(manager, racine, nombre_variables);
    clock_gettime(CLOCK_MONOTONIC, &fin);
    Cudd_RecursiveDeref(manager, racine);
    free(phase);
    free(variables);
    Cudd_Quit(manager);
    if (verifie != attendu) {
        fprintf(stderr, "fenetre : vérification échouée (attendu %lu, obtenu %lu)\n",
                attendu, verifie);
        return CODE_ECHEC;
    }
    char parametres[80];
    snprintf(parametres, sizeof(parametres),
             "\"parametres\":{\"taille\":\"%lu\",\"nb_bases\":\"%lu\"}", taille, nb_bases);
    ecrire_mesure("cudd", "bdd", "fenetre", parametres,
                  temps_ms(&debut, &fin), rss_max_ko(), noeuds, verifie);
    return 0;
}

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: cudd_bench <charge> [paramètres…]\n");
        return CODE_USAGE;
    }
    const char *charge = argv[1];
    if (strcmp(charge, "alea-lcg") == 0 && argc == 3) {
        return charge_alea_lcg(atoi(argv[2]));
    }
    if (strcmp(charge, "philosophes") == 0 && argc == 4) {
        return charge_philosophes(atoi(argv[2]), argv[3]);
    }
    if (strcmp(charge, "fenetre") == 0 && argc == 4) {
        return charge_fenetre((unsigned long)atol(argv[2]), (unsigned long)atol(argv[3]));
    }
    fprintf(stderr, "cudd_bench : charge ou paramètres invalides : %s\n", charge);
    return CODE_USAGE;
}

#include <stdio.h>
#include <stdlib.h>
#include <stddef.h>
#include <string.h>
#include <time.h>
#include <cudd.h>

/*
 * Full measurement witness for CUDD (BENCH-1, issue #277): same input
 * protocol as cudd_count (stdin: n, then 2^n values 0/1), but writes one
 * JSON line of the confrontation protocol on stdout, with the same key
 * order as lisage's serialiser_ligne:
 *   outil, variante, charge, parametres, temps_ms, rss_max_ko, noeuds, verifie
 *
 * Measured block (protocol): input reading + diagram construction +
 * verification counting. Library initialization (manager, variables) is
 * outside the block, as at the other witnesses. Peak RSS is the
 * whole-process VmHWM (one fresh container per measurement).
 *
 * Node count in the lisage convention (MEM-6, statement C6): distinct
 * (node, complement-bit) pairs reachable from the root, terminals
 * included — see cudd_count.c for the rationale.
 */

/* Open-addressing hash set of (node, complement) pairs: key = pointer*2 +
 * complement bit, stored as key+1 so that 0 means empty. Grown by doubling
 * (rehash) when the load factor exceeds 1/2, so the memory stays
 * proportional to the actual pair count (~1.4M pairs at alea-lcg-24). */
static unsigned long long *seen_set = NULL;
static unsigned long seen_size = 0; /* number of slots, a power of two */
static unsigned long seen_count = 0;

static void seen_init(unsigned long taille_minimale) {
    seen_size = 16;
    while (seen_size < taille_minimale * 2) seen_size <<= 1;
    seen_set = (unsigned long long *)calloc(seen_size, sizeof(unsigned long long));
    if (!seen_set) { fprintf(stderr, "cudd_measure: seen set allocation failed\n"); exit(2); }
}

static void seen_rehash(unsigned long nouvelle_taille) {
    unsigned long long *ancien = seen_set;
    unsigned long ancienne_taille = seen_size;
    seen_set = (unsigned long long *)calloc(nouvelle_taille, sizeof(unsigned long long));
    if (!seen_set) { fprintf(stderr, "cudd_measure: seen set rehash failed\n"); exit(2); }
    seen_size = nouvelle_taille;
    for (unsigned long i = 0; i < ancienne_taille; i++) {
        unsigned long long entry = ancien[i];
        if (entry == 0) continue;
        unsigned long long key = entry - 1;
        unsigned long slot = (unsigned long)((key * 0x9E3779B97F4A7C15ULL) >> (64 - __builtin_ctzll(nouvelle_taille)));
        while (seen_set[slot] != 0) slot = (slot + 1) & (nouvelle_taille - 1);
        seen_set[slot] = entry;
    }
    free(ancien);
}

/* Returns 1 if the (node, complement) pair is new, 0 if already seen. */
static int mark_pair(DdNode *node, int complement) {
    unsigned long long key = ((unsigned long long)(uintptr_t)node << 1) | (unsigned long long)complement;
    if ((seen_count + 1) * 2 > seen_size) seen_rehash(seen_size * 2);
    unsigned long masque = seen_size - 1;
    unsigned long slot = (unsigned long)((key * 0x9E3779B97F4A7C15ULL) >> (64 - __builtin_ctzll(seen_size)));
    for (;;) {
        unsigned long long entry = seen_set[slot & masque];
        if (entry == 0) {
            seen_set[slot & masque] = key + 1;
            seen_count++;
            return 1;
        }
        if (entry == key + 1) return 0;
        slot++;
    }
}

static void count_pairs(DdNode *f) {
    DdNode *regular = Cudd_Regular(f);
    int complement = Cudd_IsComplement(f);
    if (!mark_pair(regular, complement)) return;
    if (Cudd_IsConstant(regular)) return;
    DdNode *then_child = Cudd_T(regular);
    DdNode *else_child = Cudd_E(regular);
    if (complement) {
        count_pairs(Cudd_Not(then_child));
        count_pairs(Cudd_Not(else_child));
    } else {
        count_pairs(then_child);
        count_pairs(else_child);
    }
}

static DdNode *build_from_table(DdManager *m, DdNode **vars, int n, int *table, int niveau) {
    if (niveau == n) return table[0] ? Cudd_ReadOne(m) : Cudd_Not(Cudd_ReadOne(m));
    int moitie = 1 << (n - niveau - 1);
    DdNode *low = build_from_table(m, vars, n, table, niveau + 1);
    DdNode *high = build_from_table(m, vars, n, table + moitie, niveau + 1);
    /* Same convention as cudd_count / lisage: variable l = bit l of the
     * table index, variable 0 at the root. The result is referenced: the
     * unique-table rehash on the large charges triggers a garbage collect,
     * which must not see the intermediate diagrams as dead. */
    DdNode *resultat = Cudd_bddIte(m, vars[n - 1 - niveau], high, low);
    Cudd_Ref(resultat);
    return resultat;
}

/* Peak process RSS in kibibytes (VmHWM of /proc/self/status). */
static long rss_max_ko(void) {
    FILE *status = fopen("/proc/self/status", "r");
    if (!status) { fprintf(stderr, "cudd_measure: /proc/self/status unreadable\n"); exit(2); }
    char line[256];
    long rss = -1;
    while (fgets(line, sizeof line, status)) {
        if (strncmp(line, "VmHWM:", 6) == 0) {
            rss = strtol(line + 6, NULL, 10);
            break;
        }
    }
    fclose(status);
    if (rss < 0) { fprintf(stderr, "cudd_measure: VmHWM absent\n"); exit(2); }
    return rss;
}

static double now_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec * 1000.0 + ts.tv_nsec / 1.0e6;
}

int main(int argc, char **argv) {
    const char *charge = argc > 1 ? argv[1] : "stdin";
    /* Library initialization, outside the measured block. */
    int n;
    DdManager *manager = NULL;
    DdNode **vars = NULL;
    /* Peek at n first: the manager needs the variable count at init. The
     * peek is part of input reading (measured block starts after it). */
    if (scanf("%d", &n) != 1) return 1;
    manager = Cudd_Init(n, 0, CUDD_UNIQUE_SLOTS, CUDD_CACHE_SLOTS, 0);
    vars = (DdNode **)malloc(n * sizeof(DdNode *));
    for (int i = 0; i < n; i++) vars[i] = Cudd_bddNewVar(manager);

    /* Measured block: input reading + construction + verification counting. */
    double debut = now_ms();
    int taille = 1 << n;
    seen_init((unsigned long)taille / (unsigned long)(n > 0 ? n : 1) + 16);
    int *table = (int *)malloc(taille * sizeof(int));
    long verifie = 0;
    for (int i = 0; i < taille; i++) {
        if (scanf("%d", &table[i]) != 1) return 1;
        verifie += table[i];
    }
    DdNode *f = build_from_table(manager, vars, n, table, 0);
    /* Reference the root: the unique-table rehash triggered on the large
     * charges runs a garbage collect, which must not see the diagram as
     * dead. */
    Cudd_Ref(f);
    count_pairs(f);
    double temps_ms = now_ms() - debut;
    long rss = rss_max_ko();

    printf("{\"outil\":\"cudd\",\"variante\":\"bdd\",\"charge\":\"%s\",\"parametres\":{\"n\":\"%d\"},\"temps_ms\":%.3f,\"rss_max_ko\":%ld,\"noeuds\":%lu,\"verifie\":%ld}\n",
           charge, n, temps_ms, rss, seen_count, verifie);
    fflush(stdout);
    Cudd_RecursiveDeref(manager, f);
    return 0;
}

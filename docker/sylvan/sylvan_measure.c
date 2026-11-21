#include <stdio.h>
#include <stdlib.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>
#include <time.h>
#include <sylvan.h>

/*
 * Full measurement witness for Sylvan (issue #308, closes SP-2 #154):
 * same input protocol as the other witnesses (stdin: n, then 2^n values
 * 0/1), but writes one JSON line of the confrontation protocol on stdout,
 * with the same key order as lisage's serialiser_ligne:
 *   outil, variante, charge, parametres, temps_ms, rss_max_ko, noeuds, verifie
 *
 * Measured block (protocol): input reading + diagram construction +
 * verification counting. Library initialization (lace_start +
 * sylvan_init_package + sylvan_init_bdd) is outside the block, as at the
 * other witnesses. Peak RSS is the whole-process VmHWM (one fresh
 * container per measurement).
 *
 * Lace note: the main thread is not a Lace worker after lace_start, and
 * Sylvan's unique table dereferences lace_get_worker() on the calling
 * thread. The measured block therefore runs inside a Lace task (RUN), as
 * in Sylvan's own examples.
 *
 * Conventions: bit l of the table index = variable l, variable 0 at the
 * root (same as libddd_measure / lisage's compiler_bdd). The diagram is
 * built by bottom-up Shannon expansion with sylvan_ite(ithvar(level),
 * high, low). Sylvan BDDs carry a complement edge (sylvan_not flips
 * sylvan_complement), so `noeuds` is counted in the lisage convention like
 * the CUDD witness: distinct (node, complement-bit) pairs reachable from
 * the root, terminals included.
 *
 * GC safety: every non-terminal node of the layer being built is
 * sylvan_protect'ed before the previous layer is released, so a garbage
 * collection triggered by a full unique table never collects live nodes.
 */

/* Open-addressing hash set of (node, complement) pairs: key = node*2 +
 * complement bit, stored as key+1 so that 0 means empty. Grown by doubling
 * (rehash) when the load factor exceeds 1/2, so the memory stays
 * proportional to the actual pair count. */
static unsigned long long *seen_set = NULL;
static unsigned long seen_size = 0; /* number of slots, a power of two */
static unsigned long seen_count = 0;

static void seen_init(unsigned long taille_minimale) {
    seen_size = 16;
    while (seen_size < taille_minimale * 2) seen_size <<= 1;
    seen_set = (unsigned long long *)calloc(seen_size, sizeof(unsigned long long));
    if (!seen_set) { fprintf(stderr, "sylvan_measure: seen set allocation failed\n"); exit(2); }
}

static void seen_rehash(unsigned long nouvelle_taille) {
    unsigned long long *ancien = seen_set;
    unsigned long ancienne_taille = seen_size;
    seen_set = (unsigned long long *)calloc(nouvelle_taille, sizeof(unsigned long long));
    if (!seen_set) { fprintf(stderr, "sylvan_measure: seen set rehash failed\n"); exit(2); }
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
static int mark_pair(MTBDD node, int complement) {
    unsigned long long key = ((unsigned long long)node << 1) | (unsigned long long)complement;
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

static void count_pairs(BDD f) {
    MTBDD regular = f & ~sylvan_complement;
    int complement = (f & sylvan_complement) != 0;
    if (!mark_pair(regular, complement)) return;
    if (!mtbdd_isnode(regular)) return;
    /* mtbdd_getlow/gethigh take the complement edge into account. */
    count_pairs(mtbdd_getlow(f));
    count_pairs(mtbdd_gethigh(f));
}

/* Peak process RSS in kibibytes (VmHWM of /proc/self/status). */
static long rss_max_ko(void) {
    FILE *status = fopen("/proc/self/status", "r");
    if (!status) { fprintf(stderr, "sylvan_measure: /proc/self/status unreadable\n"); exit(2); }
    char line[256];
    long rss = -1;
    while (fgets(line, sizeof line, status)) {
        if (strncmp(line, "VmHWM:", 6) == 0) {
            rss = strtol(line + 6, NULL, 10);
            break;
        }
    }
    fclose(status);
    if (rss < 0) { fprintf(stderr, "sylvan_measure: VmHWM absent\n"); exit(2); }
    return rss;
}

static double now_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec * 1000.0 + ts.tv_nsec / 1.0e6;
}

/* Results of the measured block, produced inside the Lace task. */
static int g_n = 0;
static long g_verifie = 0;
static double g_temps_ms = 0.0;

VOID_TASK_0(measure_task)
{
    /* Measured block: input reading + construction + verification counting. */
    double debut = now_ms();
    if (scanf("%d", &g_n) != 1) exit(1);
    int n = g_n;
    size_t taille = (size_t)1 << n;
    seen_init(taille / (unsigned long)(n > 0 ? n : 1) + 16);
    int *table = (int *)malloc(taille * sizeof(int));
    for (size_t i = 0; i < taille; i++) {
        if (scanf("%d", &table[i]) != 1) exit(1);
        g_verifie += table[i];
    }
    /* Bottom-up Shannon expansion: the layer starts as the 2^n constants,
     * then each level combines pairs into sylvan_ite(var, high, low). */
    BDD *layer = (BDD *)malloc(taille * sizeof(BDD));
    for (size_t i = 0; i < taille; i++)
        layer[i] = table[i] ? sylvan_true : sylvan_false;
    for (int level = n - 1; level >= 0; level--) {
        size_t stride = (size_t)1 << level;
        BDD *next = (BDD *)malloc(stride * sizeof(BDD));
        for (size_t j = 0; j < stride; j++) {
            BDD bas = layer[j];
            BDD haut = layer[j + stride];
            next[j] = sylvan_ite(sylvan_ithvar((uint32_t)level), haut, bas);
        }
        /* Protect the new layer, then release the previous one. */
        for (size_t j = 0; j < stride; j++)
            if (mtbdd_isnode(next[j])) sylvan_protect(&next[j]);
        for (size_t j = 0; j < (size_t)1 << (level + 1); j++)
            if (mtbdd_isnode(layer[j])) sylvan_unprotect(&layer[j]);
        free(layer);
        layer = next;
    }
    BDD f = layer[0];
    sylvan_protect(&f);
    count_pairs(f);
    g_temps_ms = now_ms() - debut;
}

int main(int argc, char **argv) {
    const char *charge = argc > 1 ? argv[1] : "stdin";
    /* Library initialization, outside the measured block. Lace (the task
     * runtime) must be started before sylvan_init_package: llmsset_create
     * runs a parallel task on it. The unique table is sized so that the
     * crossover charges (alea-lcg-24 ~ 1.4M nodes) fit without a resize;
     * GC safety during construction is ensured by sylvan_protect on the
     * live layer. */
    lace_start(0, 1000000); /* 0 workers = autodetect */
    sylvan_set_sizes(1UL << 20, 1UL << 26, 1UL << 18, 1UL << 22);
    sylvan_init_package();
    sylvan_init_bdd();

    RUN(measure_task);

    long rss = rss_max_ko();
    printf("{\"outil\":\"sylvan\",\"variante\":\"bdd\",\"charge\":\"%s\",\"parametres\":{\"n\":\"%d\"},\"temps_ms\":%.3f,\"rss_max_ko\":%ld,\"noeuds\":%lu,\"verifie\":%ld}\n",
           charge, g_n, g_temps_ms, rss, seen_count, g_verifie);
    fflush(stdout);
    sylvan_quit();
    lace_stop();
    return 0;
}

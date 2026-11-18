#include <stdio.h>
#include <stdlib.h>
#include <stddef.h>
#include <cudd.h>

/*
 * Node count in the lisage convention (MEM-6, statement C6): distinct
 * (node, complement-bit) pairs reachable from the root, terminals
 * included. CUDD shares nodes through complement edges and Cudd_DagSize
 * counts each shared node once plus a single constant, so the raw
 * DagSize is NOT comparable with lisage's count_nodes; counting
 * (node, complement) pairs removes the sharing and counts both
 * terminals, which reproduces the lisage convention exactly.
 */

#define MAX_PAIRS (1 << 20)

static DdNode *seen_nodes[MAX_PAIRS];
static unsigned char seen_complement[MAX_PAIRS];
static int seen_count = 0;

/* Returns 1 if the (node, complement) pair is new, 0 if already seen. */
static int mark_pair(DdNode *node, int complement) {
    for (int i = 0; i < seen_count; i++) {
        if (seen_nodes[i] == node && seen_complement[i] == (unsigned char)complement) return 0;
    }
    if (seen_count >= MAX_PAIRS) {
        fprintf(stderr, "cudd_count: pair table overflow\n");
        exit(2);
    }
    seen_nodes[seen_count] = node;
    seen_complement[seen_count] = (unsigned char)complement;
    seen_count++;
    return 1;
}

static void count_pairs(DdNode *f) {
    DdNode *regular = Cudd_Regular(f);
    int complement = Cudd_IsComplement(f);
    if (!mark_pair(regular, complement)) return;
    if (Cudd_IsConstant(regular)) return;
    DdNode *then_child = Cudd_T(regular);
    DdNode *else_child = Cudd_E(regular);
    if (complement) {
        /* f = NOT g: the arcs of f are the complemented arcs of g. */
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
    /* The half-split at depth `niveau` isolates bit (n - 1 - niveau) of
     * the table index; assign it to variable (n - 1 - niveau) so that
     * variable l = bit l of the index, variable 0 at the root — the
     * same convention as lisage (CUDD puts index 0 at the root). */
    return Cudd_bddIte(m, vars[n - 1 - niveau], high, low);
}

int main(void) {
    int n;
    if (scanf("%d", &n) != 1) return 1;
    int taille = 1 << n;
    DdManager *manager = Cudd_Init(n, 0, CUDD_UNIQUE_SLOTS, CUDD_CACHE_SLOTS, 0);
    DdNode **vars = (DdNode **)malloc(n * sizeof(DdNode *));
    for (int i = 0; i < n; i++) vars[i] = Cudd_bddNewVar(manager);
    int *table = (int *)malloc(taille * sizeof(int));
    for (int i = 0; i < taille; i++) { if (scanf("%d", &table[i]) != 1) return 1; }
    DdNode *f = build_from_table(manager, vars, n, table, 0);
    count_pairs(f);
    printf("%d\n", seen_count);
    Cudd_Quit(manager);
    return 0;
}

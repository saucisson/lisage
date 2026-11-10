#include <stdio.h>
#include <stdlib.h>
#include <stddef.h>
#include <cudd.h>

static DdNode *build_from_table(DdManager *m, DdNode **vars, int n, int *table, int niveau) {
    if (niveau == n) return table[0] ? Cudd_ReadOne(m) : Cudd_Not(Cudd_ReadOne(m));
    int moitie = 1 << (n - niveau - 1);
    DdNode *low = build_from_table(m, vars, n, table, niveau + 1);
    DdNode *high = build_from_table(m, vars, n, table + moitie, niveau + 1);
    return Cudd_bddIte(m, vars[niveau], high, low);
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
    printf("%d\n", Cudd_DagSize(f));
    Cudd_Quit(manager);
    return 0;
}

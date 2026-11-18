// Full measurement witness for libDDD (LIP6) — BENCH-1, issue #277,
// ADR-0102 pattern (local Docker only, never in CI).
//
// Reads on stdin: `n` (number of variables) then `2^n` values `0`/`1`
// (truth table, same input format as `cudd_count` / `oxidd_count`).
// Optional argv[1]: the charge label serialized in the JSON line.
// Writes on stdout one JSON line of the confrontation protocol (same key
// order as lisage's `serialiser_ligne`):
//   outil, variante, charge, parametres, temps_ms, rss_max_ko, noeuds, verifie
//
// Measured block (protocol): input reading + diagram construction +
// verification counting. Library initialization is outside the block.
// Peak RSS is the whole-process `VmHWM` (one fresh container per
// measurement, orchestrator convention).
//
// Conventions: bit l of the table index = variable l, variable 0 at the
// root (libDDD orders variables increasingly from the root, same as
// lisage's `compiler_bdd`). The diagram is built by bottom-up Shannon
// expansion with direct node construction (DDD(var,1,high)+DDD(var,0,low))
// memoized on (level, low, high) — the generic DDD algebra operators are
// not used. `noeuds` is the lisage convention (distinct reachable nodes,
// terminals included): libDDD drops null edges, so `GDDD::size()` counts
// the `one` terminal only when reachable; the `null` terminal is added
// back whenever the function is not constant true.
// `verifie` = number of satisfying assignments (count of 1s in the table).

#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <iostream>
#include <map>
#include <vector>

#include "ddd/DDD.h"
#include "ddd/MemoryManager.h"

/// Peak process RSS in kibibytes (`VmHWM` of `/proc/self/status`).
static long rss_max_ko() {
    FILE *status = fopen("/proc/self/status", "r");
    if (!status) { fprintf(stderr, "libddd_measure: /proc/self/status unreadable\n"); exit(2); }
    char line[256];
    long rss = -1;
    while (fgets(line, sizeof line, status)) {
        if (strncmp(line, "VmHWM:", 6) == 0) {
            rss = strtol(line + 6, NULL, 10);
            break;
        }
    }
    fclose(status);
    if (rss < 0) { fprintf(stderr, "libddd_measure: VmHWM absent\n"); exit(2); }
    return rss;
}

/// Memo key of a Shannon node: (level, low, high). GDDD has a total order
/// (`operator<`, unicity of the concrete node), so a std::map suffices.
struct CleNoeud {
    int niveau;
    DDD bas;
    DDD haut;
};

struct CompareCleNoeud {
    bool operator()(const CleNoeud &a, const CleNoeud &b) const {
        if (a.niveau != b.niveau) return a.niveau < b.niveau;
        if (a.bas != b.bas) return a.bas < b.bas;
        return a.haut < b.haut;
    }
};

int main(int argc, char **argv) {
    const char *charge = argc > 1 ? argv[1] : "stdin";
    // Library initialization (static initializers of libDDD), outside the
    // measured block.

    // Measured block: input reading + construction + verification counting.
    auto debut = std::chrono::steady_clock::now();
    int n;
    if (!(std::cin >> n)) return 1;
    size_t taille = 1ul << n;
    std::vector<int> table(taille);
    long verifie = 0;
    for (size_t i = 0; i < taille; i++) {
        if (!(std::cin >> table[i])) return 1;
        verifie += table[i];
    }
    // Bottom-up Shannon expansion: layer starts as the 2^n constants,
    // then each level combines pairs into a memoized node
    // DDD(level,1,high) + DDD(level,0,low).
    std::vector<DDD> layer(taille);
    for (size_t i = 0; i < taille; i++)
        layer[i] = table[i] ? DDD(GDDD::one) : DDD(GDDD::null);
    std::map<CleNoeud, DDD, CompareCleNoeud> memoire;
    for (int level = n - 1; level >= 0; level--) {
        size_t stride = 1ul << level;
        std::vector<DDD> next(stride);
        for (size_t j = 0; j < stride; j++) {
            const DDD &bas = layer[j];
            const DDD &haut = layer[j + stride];
            if (bas == haut) {
                next[j] = bas;
                continue;
            }
            CleNoeud cle{level, bas, haut};
            auto trouve = memoire.find(cle);
            if (trouve != memoire.end()) {
                next[j] = trouve->second;
            } else {
                DDD noeud = DDD(level, 1, haut) + DDD(level, 0, bas);
                memoire.emplace(cle, noeud);
                next[j] = noeud;
            }
        }
        layer = std::move(next);
    }
    const DDD &f = layer[0];
    // Node count in the lisage convention: distinct reachable nodes,
    // terminals included. libDDD drops null edges, so size() misses the
    // null terminal unless explicitly referenced: add it back when the
    // function is not constant true.
    unsigned long noeuds = f.size() + (f == GDDD::one ? 0ul : 1ul);
    double temps_ms = std::chrono::duration<double, std::milli>(std::chrono::steady_clock::now() - debut).count();
    long rss = rss_max_ko();

    printf("{\"outil\":\"libddd\",\"variante\":\"bdd\",\"charge\":\"%s\",\"parametres\":{\"n\":\"%d\"},\"temps_ms\":%.3f,\"rss_max_ko\":%ld,\"noeuds\":%lu,\"verifie\":%ld}\n",
           charge, n, temps_ms, rss, noeuds, verifie);
    return 0;
}

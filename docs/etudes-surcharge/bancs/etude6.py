#!/usr/bin/env python3
# Étude « surcharge » partie VI — la mémoire : exigible, mais fragile
#
# Deux proxys opposés, repérés dans le catalogue MCC (Referendum, NQueens),
# où la pathologie n'est ni la couture ni le span de garde, mais l'information
# qui n'est pas dans la fenêtre :
#
#  A. Référendum : dynamique libre, requête globale. N votants en anneau
#     (topologie de la partie I), chaque votant passe de « non-voté » (0) à
#     oui (1) ou non (2), vote figé. La dynamique est sans garde ; la requête
#     « conclure » (accepté si oui strictement majoritaire, refusé sinon,
#     exige que tous aient voté) est un agrégat de comptage sur toute la
#     configuration.
#     Moteurs : plat = MDD du langage seul ; plat+requête = MDD du langage
#     avec la conclusion intégrée (une variable à 3 valeurs : 0 = délibération,
#     1 = accepté, 2 = refusé) ; compteur = (N+1)(N+2)/2 classes de
#     (votes émis, oui), l'équivalent sémantique d'arcs valués (EV-MDD).
#     Types locaux = triplets de voisinage (gauche, soi, droite) :
#     multiset (avec multiplicités) vs set (sans).
#
#  B. N-reines : garde filtrante globale, requête triviale. Grille N×N,
#     poser une reine exige l'absence de conflit ligne, colonne ou diagonale
#     avec toute reine présente (portée N, non bornée) ; retrait libre.
#     Moteurs : plat = MDD positionnel exact, en ordre ligne-par-ligne puis
#     en balayage anti-diagonal ; pool = fenêtre bornée (caps W=2 des 4
#     bords + les 8 voisins de Moore) ; erreurs = faux positifs de la
#     fenêtre (elle croit un ajout légal, la sémantique le refuse) ;
#     empreinte = registre d'occupation (lignes, colonnes, deux diagonales).
#     Courbe des portées : taux d'erreur d'une fenêtre de Chebyshev de rayon R.
#
# Usage : python3 etude6.py <referendum|nqueens|portees> <n_max>
from collections import deque
import json, sys, time, resource, os

HERE = os.path.dirname(os.path.abspath(__file__))
RESULTS = HERE + "/results6.json"
W = 2
SOLUTIONS = {1: 1, 2: 0, 3: 0, 4: 2, 5: 10, 6: 4, 7: 40, 8: 92}

def rss():
    return resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024.0

def save(key, rows):
    try:
        with open(RESULTS) as f:
            d = json.load(f)
    except Exception:
        d = {}
    d[key] = rows
    with open(RESULTS, "w") as f:
        json.dump(d, f, indent=1)

def bfs(init, succ):
    seen = {init}
    todo = deque([init])
    while todo:
        st = todo.popleft()
        for t in succ(st):
            if t not in seen:
                seen.add(t)
                todo.append(t)
    return seen

def mdd(states):
    """MDD exact positionnel : chaque nœud = une classe de suffixes."""
    nvars = len(next(iter(states)))
    ids = {}
    def build(pos, sset):
        if pos == nvars:
            return "T"
        groups = {}
        for s in sset:
            g = groups.get(s[pos])
            if g is None:
                groups[s[pos]] = g = set()
            g.add(s)
        children = tuple((v, build(pos + 1, g)) for v, g in sorted(groups.items()))
        key = (pos, children)
        r = ids.get(key)
        if r is None:
            r = ids[key] = len(ids)
        return r
    build(0, states)
    return len(ids)

# ----------------------------- A. référendum -----------------------------
def verdict(v):
    """Accepté si les oui sont strictement majoritaires (tous ont voté)."""
    return 1 if v.count(1) > v.count(2) else 2

def succ_scrutin(N):
    def f(s):
        out = []
        if s[N] == 0:
            for i in range(N):
                if s[i] == 0:
                    for x in (1, 2):
                        c = bytearray(s)
                        c[i] = x
                        out.append(bytes(c))
            if all(s[i] for i in range(N)):
                c = bytearray(s)
                c[N] = verdict(s)
                out.append(bytes(c))
        return out
    return f

def run_referendum(n_max):
    rows = []
    for N in range(2, n_max + 1):
        t0 = time.time()
        S2 = bfs(bytes(N + 1), succ_scrutin(N))
        S = {s[:N] for s in S2 if s[N] == 0}
        assert len(S) == 3 ** N, (N, len(S))
        assert len(S2) == 3 ** N + 2 ** N, (N, len(S2))
        plat = mdd(S)
        plat_req = mdd(S2)
        compteur = (N + 1) * (N + 2) // 2
        # types locaux : multiset (correct) vs set (perte de multiplicités)
        # compte = nombre de oui ; décision = « refusé encore possible »
        # (un membre « adopté certain » : o > n + a interdit tout renversement)
        classes_ms, classes_set = {}, {}
        for v in S:
            trips = [(v[(i - 1) % N], v[i], v[(i + 1) % N]) for i in range(N)]
            ms = tuple(sorted(trips))
            ss = frozenset(trips)
            o, n = v.count(1), v.count(2)
            classes_ms.setdefault(ms, set()).add(o)
            classes_set.setdefault(ss, set()).add((o, n + (N - o - n) >= o))
        assert all(len(m) == 1 for m in classes_ms.values()), \
            "le multiset doit donner le compte exact (marge centrale)"
        pool_ms, pool_set = len(classes_ms), len(classes_set)
        amb = dec_amb = k_max = 0
        for members in classes_set.values():
            if len({c[0] for c in members}) > 1:
                amb += 1
                k_max = max(k_max, len({c[0] for c in members}))
                # décision ambiguë : la classe mélange un membre « adopté
                # certain » (refusé impossible) et un membre renversable
                if len({c[1] for c in members}) > 1:
                    dec_amb += 1
        row = {"N": N, "states": 3 ** N, "plat": plat, "plat_req": plat_req,
               "compteur": compteur, "pool_ms": pool_ms, "pool_set": pool_set,
               "amb": amb, "dec_amb": dec_amb, "k_max": k_max,
               "sec": round(time.time() - t0, 1)}
        rows.append(row)
        print(json.dumps(row), flush=True)
        save("referendum", rows)
    print(f"{'N':>2} {'|S|':>7} {'plat':>4} {'+req':>4} {'cpt':>4} {'ms':>6} {'set':>6} {'amb':>4} {'dec':>4} {'k':>2}")
    for r in rows:
        print(f"{r['N']:>2} {r['states']:>7} {r['plat']:>4} {r['plat_req']:>4} {r['compteur']:>4} {r['pool_ms']:>6} {r['pool_set']:>6} {r['amb']:>4} {r['dec_amb']:>4} {r['k_max']:>2}")

# ----------------------------- B. N-reines -----------------------------
def conflits(n):
    """Cellules en conflit (ligne, colonne, deux diagonales) pour chaque p."""
    confl = []
    for p in range(n * n):
        i, j = divmod(p, n)
        s = set()
        for q in range(n * n):
            if q == p:
                continue
            a, b = divmod(q, n)
            if a == i or b == j or (a - b) == (i - j) or (a + b) == (i + j):
                s.add(q)
        confl.append(frozenset(s))
    return confl

def voisins8(n):
    v8 = []
    for p in range(n * n):
        i, j = divmod(p, n)
        s = []
        for di in (-1, 0, 1):
            for dj in (-1, 0, 1):
                if di == 0 and dj == 0:
                    continue
                a, b = i + di, j + dj
                if 0 <= a < n and 0 <= b < n:
                    s.append(a * n + b)
        v8.append(tuple(s))
    return v8

def succ_reines(n, confl):
    def f(s):
        out = []
        for p in range(n * n):
            if s[p]:
                c = bytearray(s)
                c[p] = 0
                out.append(bytes(c))
            elif not any(s[q] for q in confl[p]):
                c = bytearray(s)
                c[p] = 1
                out.append(bytes(c))
        return out
    return f

def empreinte(s, n):
    rows, cols, d1, d2 = [], [], [], []
    for p in range(n * n):
        if s[p]:
            i, j = divmod(p, n)
            rows.append(i)
            cols.append(j)
            d1.append(i - j)
            d2.append(i + j)
    return (frozenset(rows), frozenset(cols), frozenset(d1), frozenset(d2))

def run_nqueens(n_max):
    rows = []
    for n in range(4, n_max + 1):
        t0 = time.time()
        confl = conflits(n)
        v8 = voisins8(n)
        S = bfs(bytes(n * n), succ_reines(n, confl))
        pleines = sum(1 for s in S if sum(s) == n)
        assert pleines == SOLUTIONS[n], (n, pleines)
        plat_ligne = mdd(S)
        # ordre de balayage anti-diagonal
        perm = sorted(range(n * n), key=lambda p: (p // n + p % n, p % n))
        Sd = {bytes(s[perm[q]] for q in range(n * n)) for s in S}
        plat_diag = mdd(Sd)
        # pool : fenêtre bornée (caps des 4 bords, cellule, les 8 voisins)
        sigs = set()
        for s in S:
            for p in range(n * n):
                i, j = divmod(p, n)
                sigs.add((min(i, W), min(n - 1 - i, W), min(j, W), min(n - 1 - j, W),
                          s[p], tuple(s[q] for q in v8[p])))
        # erreurs : part des paires (configuration, cellule) mal prédites par
        # la fenêtre : elle croit un ajout légal, la sémantique le refuse
        FP = E = 0
        for s in S:
            for p in range(n * n):
                if s[p]:
                    continue
                E += 1
                if not any(s[q] for q in v8[p]):
                    if any(s[q] for q in confl[p]):
                        FP += 1
        # empreinte d'occupation
        emp = {empreinte(s, n) for s in S}
        row = {"n": n, "states": len(S), "pleines": pleines,
               "plat_ligne": plat_ligne, "plat_diag": plat_diag,
               "pool": len(sigs), "err_pct": round(100.0 * FP / E, 2),
               "empreintes": len(emp), "sec": round(time.time() - t0, 1)}
        rows.append(row)
        print(json.dumps(row), flush=True)
        save("nqueens", rows)
    print(f"{'N':>2} {'|S|':>7} {'pl.':>3} {'ligne':>7} {'diag':>7} {'pool':>5} {'err%':>6} {'emp.':>6}")
    for r in rows:
        print(f"{r['n']:>2} {r['states']:>7} {r['pleines']:>3} {r['plat_ligne']:>7} {r['plat_diag']:>7} {r['pool']:>5} {r['err_pct']:>6} {r['empreintes']:>6}")

def fenetre_portees(N):
    """Courbe du taux de faux positifs d'une fenêtre de Chebyshev de rayon R."""
    confl = conflits(N)
    S = bfs(bytes(N * N), succ_reines(N, confl))
    out = {}
    for R in range(1, N):
        win = []
        for p in range(N * N):
            i, j = divmod(p, N)
            w = []
            for di in range(-R, R + 1):
                for dj in range(-R, R + 1):
                    a, b = i + di, j + dj
                    if 0 <= a < N and 0 <= b < N:
                        w.append(a * N + b)
            win.append(tuple(w))
        FP = E = 0
        for s in S:
            for p in range(N * N):
                if s[p]:
                    continue
                E += 1
                if not any(s[q] for q in win[p]):
                    if any(s[q] for q in confl[p]):
                        FP += 1
        out[R] = round(100.0 * FP / E, 2)
    print(json.dumps({"N": N, "portees": out}), flush=True)
    save(f"portees-{N}", {"N": N, "portees": {str(k): v for k, v in out.items()}})
    return out

if __name__ == "__main__":
    mode = sys.argv[1]
    nmax = int(sys.argv[2])
    if mode == "referendum":
        run_referendum(nmax)
    elif mode == "portees":
        fenetre_portees(nmax)
    else:
        run_nqueens(nmax)
    print("RSS max : %.0f Mo" % rss())

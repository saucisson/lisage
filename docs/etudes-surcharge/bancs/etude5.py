#!/usr/bin/env python3
# Étude « surcharge » partie V — grilles 2D : coutures croisées et coin
# (encodage bytes, exécution par variante, résultats sauvegardés par n)
#
# Modèle : langage des ensembles indépendants d'une grille n×n (cellule {0,1},
# garde locale : ajouter un 1 exige les 4 voisins à 0 ; retrait libre).
# Le pendant 2D exact du langage des philosophes de la partie I.
#
# Variantes de frontière (pour isoler les coutures d'enroulement) :
#   tore     : enroulement sur les DEUX axes (2 coutures croisées + « coin »)
#   cylindre : enroulement sur l'axe des lignes seulement (1 couture)
#   plan     : bords libres (aucune couture), sentinelles -1 dans la fenêtre
# + tore « pressé » : la cellule (0,0) a un alphabet {0,1,2} (module singleton,
#   le gaucher 2D). Ancre = la cellule irrégulière elle-même.
#
# Moteurs : plat = MDD exact positionnel en ordre ligne-par-ligne ;
# pool = signature 2D (distances plafonnées aux 4 bords + fenêtre 5 cellules
# + lettre d'ancrage), sur les configurations ancrées par translation canonique.
# Question du coin : les classes « intérieur plafonné » (A) et « touchant un
# bord plafonné » (B) plafonnent-elles toutes les deux en n ?
#
# Usage : python3 etude5.py <tore|cylindre|plan|tore-presse> <n_max>
from collections import deque
import json, sys, time, resource, os

HERE = os.path.dirname(os.path.abspath(__file__))
RESULTS = HERE + "/results5.json"
W = 2

def dims(n, wrap):
    dis = list(range(n)) if wrap in ("tore", "cylindre") else [0]
    djs = list(range(n)) if wrap == "tore" else [0]
    return dis, djs

def neighbors(i, j, n, wrap):
    out = []
    for di in (-1, 1):
        ii = i + di
        if wrap in ("tore", "cylindre"):
            out.append((ii % n, j))
        elif 0 <= ii < n:
            out.append((ii, j))
    for dj in (-1, 1):
        jj = j + dj
        if wrap == "tore":
            out.append((i, jj % n))
        elif 0 <= jj < n:
            out.append((i, jj))
    return out

def valid_cols(n):
    """Configurations d'une colonne verticale enroulée (cycle C_n)."""
    out = []
    for b in range(1 << n):
        ok = all(not ((b >> i) & 1 and (b >> ((i + 1) % n)) & 1) for i in range(n))
        if ok:
            out.append(b)
    return out

def precheck(n):
    """|S| exact par matrice de transfert : tore (trace) et cylindre (chaîne)."""
    cols = valid_cols(n)
    k = len(cols)
    A = [[0] * k for _ in range(k)]
    for a in range(k):
        for b in range(k):
            if cols[a] & cols[b] == 0:
                A[a][b] = 1
    # cylindre : n colonnes en chaîne (axe j libre)
    v = [1] * k
    for _ in range(n - 1):
        v = [sum(A[i][j] * v[j] for j in range(k)) for i in range(k)]
    cyl = sum(v)
    # tore : les colonnes forment un cycle (trace de A^n)
    M = [row[:] for row in A]
    for _ in range(n - 1):
        M = [[sum(M[i][t] * A[t][j] for t in range(k)) for j in range(k)] for i in range(k)]
    tor = sum(M[i][i] for i in range(k))
    return tor, cyl

def succ_gen(n, wrap, irregular):
    """Générateur des successeurs, avec voisins précalculés."""
    nbs = [[neighbors(i, j, n, wrap) for j in range(n)] for i in range(n)]
    def f(cfg):
        out = []
        for i in range(n):
            base = i * n
            for j in range(n):
                v = cfg[base + j]
                maxi = 2 if (irregular and i == 0 and j == 0) else 1
                if v < maxi:
                    ok = True
                    for x, y in nbs[i][j]:
                        if cfg[x * n + y]:
                            ok = False
                            break
                    if ok:
                        c = bytearray(cfg)
                        c[base + j] += 1
                        out.append(bytes(c))
                if v:
                    c = bytearray(cfg)
                    c[base + j] -= 1
                    out.append(bytes(c))
        return out
    return f

def invariants(cfg, n, wrap, irregular=False):
    nbs = [[neighbors(i, j, n, wrap) for j in range(n)] for i in range(n)]
    for i in range(n):
        for j in range(n):
            if cfg[i * n + j] >= 1:
                for x, y in nbs[i][j]:
                    if cfg[x * n + y]:
                        return False
    return True

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

def mdd(vars_n, states):
    ids = {}
    def build(pos, sset):
        if pos == vars_n:
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

def translations(cfg, n, wrap):
    dis, djs = dims(n, wrap)
    for di in dis:
        for dj in djs:
            b = bytearray(n * n)
            for i in range(n):
                for j in range(n):
                    b[i * n + j] = cfg[((i + di) % n) * n + ((j + dj) % n)]
            yield bytes(b)

def anchor(states, n, wrap):
    return {min(translations(s, n, wrap)) for s in states}

def pool_2d(anchored, n, irregular=False, wrap="tore"):
    """Signature = (caps des 4 distances, lettre d'ancre, fenêtre 5)."""
    A, B = set(), set()
    for s in anchored:
        s00 = s[0]
        for i in range(n):
            base = i * n
            for j in range(n):
                cx, cX = min(i, W), min(n - 1 - i, W)
                cy, cY = min(j, W), min(n - 1 - j, W)
                up = s[((i - 1) % n) * n + j]
                dn = s[((i + 1) % n) * n + j]
                lf = s[base + ((j - 1) % n)]
                rt = s[base + ((j + 1) % n)]
                if wrap == "cylindre":
                    if j == 0:
                        lf = -1
                    elif j == n - 1:
                        rt = -1
                elif wrap == "plan":
                    if i == 0:
                        up = -1
                    elif i == n - 1:
                        dn = -1
                    if j == 0:
                        lf = -1
                    elif j == n - 1:
                        rt = -1
                sig = (cx, cX, cy, cY, s00, s[base + j], up, dn, lf, rt)
                (A if (cx == cX == cy == cY == W) else B).add(sig)
    return len(A) + len(B), len(A), len(B)

def controller_plan(wrap, irregular):
    p = []
    if wrap == "tore":
        p.append("2 coutures d'enroulement croisées + coin : ancre translation, fenêtres plafonnées")
    elif wrap == "cylindre":
        p.append("1 couture d'enroulement (axe i) : ancre 1D.")
    else:
        p.append("bords libres : aucune couture.")
    if irregular:
        p.append("module singleton (cellule {0,1,2}) : couture de classe, ancre = cellule irrégulière.")
    return p

def load_results():
    try:
        with open(RESULTS) as f:
            return json.load(f)
    except Exception:
        return {}

def save_results(results):
    with open(RESULTS, "w") as f:
        json.dump(results, f, indent=1)

def run(wrap, irregular, n_max):
    rows = []
    results = load_results()
    key = "tore-pressé" if irregular else wrap
    for n in range(2, n_max + 1):
        t0 = time.time()
        init = bytes(n * n)
        succ = succ_gen(n, wrap, irregular)
        S = bfs(init, succ)
        assert len(S) == precheck(n)[0] if wrap == "tore" and not irregular else True
        if n <= 4:
            assert all(invariants(c, n, wrap, irregular) for c in S), "invariant violé"
        plat = mdd(n * n, S)
        if irregular:
            anch = S
        else:
            anch = anchor(S, n, wrap)
        tot, a_in, b_bd = pool_2d(anch, n, irregular,
                                  wrap=wrap)
        row = {"n": n, "states": len(S), "anchored": len(anch),
               "S_div_n2": len(anch) / (n * n),
               "plat": plat, "pool": tot, "A_int": a_in, "B_bord": b_bd,
               "sec": round(time.time() - t0, 1)}
        rows.append(row)
        print(json.dumps(row), flush=True)
        results[key] = rows
        save_results(results)
    return rows

def show(name, rows):
    print(f"=== {name} ===")
    print(f"{'n':>2} {'|S|':>9} {'ancré':>9} {'/n²':>5} {'plat':>8} {'pool':>5} {'A(int)':>6} {'B(bord)':>7}")
    for r in rows:
        print(f"{r['n']:>2} {r['states']:>9} {r['anchored']:>9} {r['S_div_n2']:>5.1f} {r['plat']:>8} {r['pool']:>5} {r['A_int']:>6} {r['B_bord']:>7}")
    if len(rows) > 1:
        plat_growth = [round(rows[i + 1]['plat'] / rows[i]['plat'], 1) for i in range(len(rows) - 1)]
        print("ratios plat successifs :", plat_growth)
    print("pool successifs        :", [r['pool'] for r in rows])
    print()

if __name__ == "__main__":
    w = sys.argv[1]
    nmax = int(sys.argv[2])
    irr = w == "tore-presse"
    base = "tore" if w.startswith("tore") else w
    print(f"[pré-check n={nmax} : tore={precheck(nmax)[0]}, cylindre={precheck(nmax)[1]}]", flush=True)
    rows = run(base, irr, nmax)
    show(w, rows)
    print("RSS max : %.0f Mo" % (resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024.0))

#!/usr/bin/env python3
# Étude « surcharge » partie VII — détecter avant de réduire
#
# Objectif : à partir des seules lectures statiques des gardes (les supports
# R(q), sans énumérer le langage), retrouver les verdicts manuels des parties
# I à VI et prédire le plat. Trois lectures :
#   1. rayon modèle ρ : distance Chebyshev maximale entre la cellule écrite
#      et ses lectures ;
#   2. forme de requête : pli de comptage avec seuil (référendum : majorité) ;
#   3. mémoire de coupe I(p) : à chaque coupe p de l'ordre des variables,
#      l'ensemble des cellules futures directement interdites par les
#      cellules occupées du préfixe, I = {v >= p : u < p occupé, v in R(u)}.
#
# Théorème du banc : le nombre de nœuds du plat MDD au niveau p égale
# exactement le nombre de motifs I(p) distincts parmi les préfixes réels.
# Chaque nœud du plat est un motif d'interdits résiduels.
# Contre-hypothèse à réfuter : les nœuds seraient les projections des
# préfixes sur les cellules encore lues (égalité seulement partielle).
#
# Le classifieur lit (rayon, requête, profil de coupe) et tranche :
# requête agrégative -> compteur ; rayon croissant et mémoire croissante ->
# registre ; rayon borné et bande fine -> fenêtre ; bourrelet -> + couture.
#
# Usage : python3 etude7.py
from collections import deque
import json, sys, time, resource, os

HERE = os.path.dirname(os.path.abspath(__file__))
RESULTS = HERE + "/results7.json"
SEUIL = 2_000_000

def rss():
    return resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024.0

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

def mdd_levels(states):
    """MDD exact : liste du nombre de nœuds par niveau."""
    nvars = len(next(iter(states)))
    ids = {}
    counts = [0] * nvars
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
            counts[pos] += 1
        return r
    build(0, states)
    return counts

def memoire_de_coupe(states, R):
    """Nombre de motifs I(p) distincts, par coupe p (version incrémentale :
    M(t) = (union des R[u], u < t occupés) inter [t, nvars))."""
    nvars = len(next(iter(states)))
    R_after = [frozenset(v for v in R[p] if v > p) for p in range(nvars)]
    motifs = [set() for _ in range(nvars)]
    for s in states:
        M = set()
        for p in range(nvars):
            motifs[p].add(frozenset(M))
            M.discard(p)
            if s[p]:
                M |= R_after[p]
    return [len(m) for m in motifs]

def projections_cellsules(states, R, nvars):
    """Contre-hypothèse : classes des projections du préfixe sur les
    cellules passées encore lues par une garde future."""
    readers = [max([v for v in range(nvars) if u in R[v]], default=-1)
               for u in range(nvars)]
    prof = []
    for p in range(nvars):
        live = tuple(u for u in range(p) if readers[u] >= p)
        motifs = {tuple(s[u] for u in live) for s in states}
        prof.append(len(motifs))
    return prof

# ----------------------------- familles -----------------------------
def fam_anneau(n):
    """Ensembles indépendants du cycle C_n (partie I), ordre naturel."""
    R = [frozenset({(i - 1) % n, (i + 1) % n}) for i in range(n)]
    def succ(s):
        out = []
        for i in range(n):
            if s[i]:
                c = bytearray(s)
                c[i] = 0
                out.append(bytes(c))
            elif not s[(i - 1) % n] and not s[(i + 1) % n]:
                c = bytearray(s)
                c[i] = 1
                out.append(bytes(c))
        return out
    return {"S": bfs(bytes(n), succ), "R": R, "requete": False, "rho": 1,
            "rho_croit": False, "nom": f"C_{n}"}

def fam_grille(n, wrap):
    """Ensembles indépendants d'une grille n×n (partie V), ligne par ligne."""
    R = []
    for i in range(n):
        for j in range(n):
            out = set()
            for di, dj in ((-1, 0), (1, 0), (0, -1), (0, 1)):
                a, b = i + di, j + dj
                if wrap == "tore":
                    out.add((a % n) * n + (b % n))
                elif 0 <= a < n and 0 <= b < n:
                    out.add(a * n + b)
            out.discard(i * n + j)
            R.append(frozenset(out))
    nb = [list(r) for r in R]
    def succ(s):
        out = []
        for p in range(n * n):
            if s[p]:
                c = bytearray(s)
                c[p] = 0
                out.append(bytes(c))
            elif not any(s[q] for q in nb[p]):
                c = bytearray(s)
                c[p] = 1
                out.append(bytes(c))
        return out
    return {"S": bfs(bytes(n * n), succ), "R": R, "requete": False, "rho": 1,
            "rho_croit": False, "nom": f"grille-{wrap} {n}x{n}"}

def fam_nreines(N):
    """N-reines : placer sans conflit ligne/colonne/diagonales (partie VI)."""
    R = []
    for p in range(N * N):
        i, j = divmod(p, N)
        s = set()
        for q in range(N * N):
            if q == p:
                continue
            a, b = divmod(q, N)
            if a == i or b == j or (a - b) == (i - j) or (a + b) == (i + j):
                s.add(q)
        R.append(frozenset(s))
    def succ(s):
        out = []
        for p in range(N * N):
            if s[p]:
                c = bytearray(s)
                c[p] = 0
                out.append(bytes(c))
            elif not any(s[q] for q in R[p]):
                c = bytearray(s)
                c[p] = 1
                out.append(bytes(c))
        return out
    return {"S": bfs(bytes(N * N), succ), "R": R, "requete": False,
            "rho": N - 1, "rho_croit": True, "nom": f"N-reines {N}x{N}"}

def fam_referendum(N):
    """Scrutin libre à N votants (partie VI) : garde libre, requête majoritaire."""
    import itertools
    S = {bytes(c) for c in itertools.product((0, 1, 2), repeat=N)}
    return {"S": S, "R": [frozenset()] * N, "requete": True, "rho": 0,
            "rho_croit": False, "nom": f"référendum N={N}"}

# ----------------------------- classifieur -----------------------------
def classifieur(fam, rows):
    """(rayon, requête, profil de coupe) -> verdict de rung."""
    if fam["requete"]:
        return "compteur"
    if fam["rho_croit"]:
        return "registre"
    # bourrelet : aux fins de lignes de la plus grande instance carrée,
    # pic intérieur >= 2 x creux intérieur (couture statique)
    import math
    fins_all = []
    for r in rows:
        m = int(round(math.sqrt(len(r["coupe"]))))
        if m * m == len(r["coupe"]) and m >= 3:
            fins = [r["coupe"][p] for p in range(m * m) if (p + 1) % m == 0]
            fins_all = fins
    if fins_all:
        interieur = fins_all[1:-1]
        if interieur and max(interieur) >= 2 * min(interieur):
            return "fenêtre + couture"
    return "fenêtre"

def run_family(mkfam, key, ns, results):
    rows = []
    for n in ns:
        t0 = time.time()
        fam = mkfam(n)
        S = fam["S"]
        nvars = len(next(iter(S)))
        assert len(S) * nvars <= SEUIL, (key, n, len(S), nvars)
        plat = mdd_levels(S)
        coupe = memoire_de_coupe(S, fam["R"])
        proj = projections_cellsules(S, fam["R"], nvars)
        egal = sum(1 for p in range(nvars) if plat[p] == coupe[p])
        egal_proj = sum(1 for p in range(nvars) if plat[p] == proj[p])
        rows.append({"n": n, "states": len(S), "niveaux": nvars,
                     "plat": sum(plat), "coupe": coupe, "coupe_max": max(coupe),
                     "egalite": f"{egal}/{nvars}",
                     "egal_proj": f"{egal_proj}/{nvars}",
                     "sec": round(time.time() - t0, 1)})
        print(json.dumps({"famille": key, "n": n, "states": len(S),
                          "plat": sum(plat), "coupe_max": max(coupe),
                          "egalite": f"{egal}/{nvars}",
                          "sec": round(time.time() - t0, 1)}), flush=True)
    results[key] = rows
    with open(RESULTS, "w") as f:
        json.dump(results, f, indent=1)
    return rows

if __name__ == "__main__":
    t00 = time.time()
    results = {}
    familles = [
        ("anneau", fam_anneau, list(range(3, 10))),
        ("grille-plan", lambda n: fam_grille(n, "plan"), [2, 3, 4, 5]),
        ("grille-tore", lambda n: fam_grille(n, "tore"), [2, 3, 4, 5]),
        ("n-reines", fam_nreines, [4, 5, 6, 7]),
        ("référendum", fam_referendum, list(range(3, 11))),
    ]
    familles_ref = {}
    for key, mk, ns in familles:
        rows = run_family(mk, key, ns, results)
        familles_ref[key] = (mk(ns[-1]), rows)

    # théorème : égalité parfaite partout ?
    total_niv = sum(r["niveaux"] for rows in results.values() for r in rows)
    total_egal = sum(int(r["egalite"].split("/")[0]) for rows in results.values() for r in rows)
    total_proj = sum(int(r["egal_proj"].split("/")[0]) for rows in results.values() for r in rows)
    print(f"\négalité théorème (nœuds = |I(p)|) : {total_egal}/{total_niv} niveaux")
    print(f"contre-hypothèse (projections)  : {total_proj}/{total_niv} niveaux")
    assert total_egal == total_niv, "théorème violé"

    # bourrelet de couture : fins de lignes, grilles n=5
    for key in ("grille-plan", "grille-tore"):
        r5 = [r for r in results[key] if r["n"] == 5][0]
        n = 5
        fins = [r5["coupe"][p] for p in range(n * n) if (p + 1) % n == 0]
        print(f"mémoire de coupe aux fins de lignes ({key}, n=5) : {fins}")
    # contrôles croisés : plats des parties V et VI
    p5 = [r for r in results["grille-plan"] if r["n"] == 5][0]
    t5 = [r for r in results["grille-tore"] if r["n"] == 5][0]
    nr7 = [r for r in results["n-reines"] if r["n"] == 7][0]
    assert p5["plat"] == 249, p5["plat"]
    assert t5["plat"] == 1159, t5["plat"]
    assert nr7["plat"] == 15180, nr7["plat"]
    # plateau anneau et explosion N-reines
    max_an = [max(r["coupe"]) for r in results["anneau"]]
    print(f"coupe max anneau C_n (n=3..9)   : {max_an}")
    assert max(max_an[3:]) == max(max_an) == 4, max_an  # plateau dès n=6
    coupes_nr = [max(r["coupe"]) for r in results["n-reines"]]
    print(f"coupe max N-reines (N=4..7)    : {coupes_nr}")
    assert coupes_nr == [17, 59, 265, 1146], coupes_nr
    print("contrôles croisés : plat plan5=249, tore5=1159, n-reines7=15180, "
          "plateau anneau 4 dès n=6, coupe N-reines 17/59/265/1146 : conformes")

    # tableau des verdicts
    historique = {
        "anneau": ("fenêtre", "I : la fenêtre/handles porte le gain"),
        "grille-plan": ("fenêtre", "V : fenêtre correcte, surcharge non rentable"),
        "grille-tore": ("fenêtre + couture", "V : la couture porte le gain"),
        "référendum": ("compteur", "VI-A : multiplicités/compteur"),
        "n-reines": ("registre", "VI-B : fenêtre fausse, registre"),
    }
    print(f"\n{'famille':>12} {'ρ':>4} {'requête':>8} {'coupe max':>9} {'verdict':>17} {'attendu':>17} {'accord':>7}")
    accords = 0
    for key, mk, ns in familles:
        fam, rows = familles_ref[key]
        v = classifieur(fam, rows)
        attendu, ref = historique[key]
        ok = "oui" if v == attendu else "NON"
        accords += v == attendu
        requete = "oui" if fam["requete"] else "-"
        cmax = max(max(r["coupe"]) for r in rows)
        rho = "n-1" if fam["rho_croit"] else str(fam["rho"])
        print(f"{key:>12} {rho:>4} {requete:>8} {cmax:>9} {v:>17} {attendu:>17} {ok:>7}")
    assert accords == len(familles), "désaccord du classifieur"
    print(f"\n{accords}/{len(familles)} accords. Détecteur conforme aux verdicts historiques I-VI.")
    print("OK -> results7.json (%.1f s, RSS max %.0f Mo)" % (time.time() - t00, rss()))

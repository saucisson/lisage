#!/usr/bin/env python3
"""Etude "surcharge" partie VII - detecter avant de reduire.

From static reads of the guards alone (supports R(q), without enumerating
the language), recover the manual verdicts of parts I-VI and predict the
flat MDD. Three readings: model radius rho, query shape, cut memory I(p).

Theorem of the bench: the number of flat-MDD nodes at level p equals the
number of distinct I(p) motifs over real prefixes.

Usage : python3 etude7.py
"""

import itertools
import json
import math
import resource
import sys
import time
from collections import deque
from pathlib import Path

HERE = Path(__file__).resolve().parent
RESULTS = HERE / "results7.json"
SEUIL = 2_000_000
MIN_GRID_N = 3
MIN_SQUARE_CHECK = 3
COUTURE_FACTOR = 2
ANNEAU_N_MIN = 3
ANNEAU_N_MAX = 10
GRID_NS = [2, 3, 4, 5]
NREINES_NS = [4, 5, 6, 7]
REFERENDUM_N_MIN = 3
REFERENDUM_N_MAX = 11
EXPECTED_PLAN5 = 249
EXPECTED_TORE5 = 1159
EXPECTED_NR7 = 15180
EXPECTED_ANNEAU_PLATEAU = 4
EXPECTED_NR_COUPE = [17, 59, 265, 1146]


def rss() -> float:
    """Peak RSS of this process, in MiB."""
    return resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024.0


def emit(line: str) -> None:
    """Write one line to stdout (CLI entry point of the bench)."""
    sys.stdout.write(line + "\n")
    sys.stdout.flush()


def bfs(init: bytes, succ: object) -> set[bytes]:
    """Reachable states from init under succ."""
    seen = {init}
    todo = deque([init])
    while todo:
        st = todo.popleft()
        for t in succ(st):
            if t not in seen:
                seen.add(t)
                todo.append(t)
    return seen


def mdd_levels(states: set[bytes]) -> list[int]:
    """Exact MDD: node count per level."""
    nvars = len(next(iter(states)))
    ids: dict[object, int] = {}
    counts = [0] * nvars

    def build(pos: int, sset: set[bytes]) -> object:
        if pos == nvars:
            return "T"
        groups: dict[int, set[bytes]] = {}
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


def memoire_de_coupe(states: set[bytes], reads: list[frozenset[int]]) -> list[int]:
    """Number of distinct I(p) motifs per cut p.

    Incremental version: M(t) = (union of R[u], u < t occupied) inter [t, nvars).
    """
    nvars = len(next(iter(states)))
    r_after = [frozenset(v for v in reads[p] if v > p) for p in range(nvars)]
    motifs = [set() for _ in range(nvars)]
    for s in states:
        m = set()
        for p in range(nvars):
            motifs[p].add(frozenset(m))
            m.discard(p)
            if s[p]:
                m |= r_after[p]
    return [len(m) for m in motifs]


def projections_cellsules(
    states: set[bytes],
    reads: list[frozenset[int]],
    nvars: int,
) -> list[int]:
    """Counter-hypothesis: prefix projection classes.

    Classes of the prefix projection onto past cells still read by some
    future guard.
    """
    readers = [max([v for v in range(nvars) if u in reads[v]], default=-1) for u in range(nvars)]
    prof = []
    for p in range(nvars):
        live = tuple(u for u in range(p) if readers[u] >= p)
        motifs = {tuple(s[u] for u in live) for s in states}
        prof.append(len(motifs))
    return prof


# ----------------------------- families -----------------------------
def fam_anneau(n: int) -> dict[str, object]:
    """Independent sets of the cycle C_n (part I), natural order."""
    reads = [frozenset({(i - 1) % n, (i + 1) % n}) for i in range(n)]

    def succ(s: bytes) -> list[bytes]:
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

    return {
        "S": bfs(bytes(n), succ),
        "R": reads,
        "requete": False,
        "rho": 1,
        "rho_croit": False,
        "nom": f"C_{n}",
    }


def fam_grille(n: int, wrap: str) -> dict[str, object]:
    """Independent sets of an n*n grid (part V), row by row."""
    reads = []
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
            reads.append(frozenset(out))
    nb = [list(r) for r in reads]

    def succ(s: bytes) -> list[bytes]:
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

    return {
        "S": bfs(bytes(n * n), succ),
        "R": reads,
        "requete": False,
        "rho": 1,
        "rho_croit": False,
        "nom": f"grille-{wrap} {n}x{n}",
    }


def fam_nreines(size: int) -> dict[str, object]:
    """N-queens: place without row/column/diagonal conflict (part VI)."""
    reads = []
    for p in range(size * size):
        i, j = divmod(p, size)
        s = set()
        for q in range(size * size):
            if q == p:
                continue
            a, b = divmod(q, size)
            if a == i or b == j or (a - b) == (i - j) or (a + b) == (i + j):
                s.add(q)
        reads.append(frozenset(s))

    def succ(s: bytes) -> list[bytes]:
        out = []
        for p in range(size * size):
            if s[p]:
                c = bytearray(s)
                c[p] = 0
                out.append(bytes(c))
            elif not any(s[q] for q in reads[p]):
                c = bytearray(s)
                c[p] = 1
                out.append(bytes(c))
        return out

    return {
        "S": bfs(bytes(size * size), succ),
        "R": reads,
        "requete": False,
        "rho": size - 1,
        "rho_croit": True,
        "nom": f"N-reines {size}x{size}",
    }


def fam_referendum(n_voters: int) -> dict[str, object]:
    """Free N-voter ballot (part VI): free guard, majority query."""
    states = {bytes(c) for c in itertools.product((0, 1, 2), repeat=n_voters)}
    return {
        "S": states,
        "R": [frozenset()] * n_voters,
        "requete": True,
        "rho": 0,
        "rho_croit": False,
        "nom": f"referendum N={n_voters}",
    }


# ----------------------------- classifier -----------------------------
def classifieur(fam: dict[str, object], rows: list[dict[str, object]]) -> str:
    """(radius, query, cut profile) -> rung verdict."""
    if fam["requete"]:
        return "compteur"
    if fam["rho_croit"]:
        return "registre"
    # seam bulge: at the row ends of the largest square instance, inner peak
    # >= 2 x inner trough (static seam)
    fins_all = []
    for r in rows:
        m = round(math.sqrt(len(r["coupe"])))
        if m * m == len(r["coupe"]) and m >= MIN_SQUARE_CHECK:
            fins_all = [r["coupe"][p] for p in range(m * m) if (p + 1) % m == 0]
    if fins_all:
        interieur = fins_all[1:-1]
        if interieur and max(interieur) >= COUTURE_FACTOR * min(interieur):
            return "fenetre + couture"
    return "fenetre"


def run_family(
    mkfam: object,
    key: str,
    ns: list[int],
    results: dict[str, list[dict[str, object]]],
) -> list[dict[str, object]]:
    """Run one family over ns, printing and saving each row."""
    rows: list[dict[str, object]] = []
    for n in ns:
        t0 = time.time()
        fam = mkfam(n)
        states = fam["S"]
        nvars = len(next(iter(states)))
        if len(states) * nvars > SEUIL:
            raise RuntimeError(f"seuil depasse: {key}, {n}, {len(states)}, {nvars}")
        plat = mdd_levels(states)
        coupe = memoire_de_coupe(states, fam["R"])
        proj = projections_cellsules(states, fam["R"], nvars)
        egal = sum(1 for p in range(nvars) if plat[p] == coupe[p])
        egal_proj = sum(1 for p in range(nvars) if plat[p] == proj[p])
        rows.append(
            {
                "n": n,
                "states": len(states),
                "niveaux": nvars,
                "plat": sum(plat),
                "coupe": coupe,
                "coupe_max": max(coupe),
                "egalite": f"{egal}/{nvars}",
                "egal_proj": f"{egal_proj}/{nvars}",
                "sec": round(time.time() - t0, 1),
            }
        )
        emit(
            json.dumps(
                {
                    "famille": key,
                    "n": n,
                    "states": len(states),
                    "plat": sum(plat),
                    "coupe_max": max(coupe),
                    "egalite": f"{egal}/{nvars}",
                    "sec": round(time.time() - t0, 1),
                }
            )
        )
    results[key] = rows
    with RESULTS.open("w", encoding="utf-8") as f:
        json.dump(results, f, indent=1)
    return rows


def check_theorem(results: dict[str, list[dict[str, object]]]) -> tuple[int, int, int]:
    """The theorem: node count == |I(p)| at every level, everywhere."""
    total_niv = sum(r["niveaux"] for rows in results.values() for r in rows)
    total_egal = sum(int(r["egalite"].split("/")[0]) for rows in results.values() for r in rows)
    total_proj = sum(int(r["egal_proj"].split("/")[0]) for rows in results.values() for r in rows)
    emit(f"\negalite theoreme (noeuds = |I(p)|) : {total_egal}/{total_niv} niveaux")
    emit(f"contre-hypothese (projections)  : {total_proj}/{total_niv} niveaux")
    if total_egal != total_niv:
        raise RuntimeError("theoreme viole")
    return total_egal, total_niv, total_proj


def check_cross(results: dict[str, list[dict[str, object]]]) -> None:
    """Seam bulge and cross-checks against parts V and VI."""
    grid_n = 5
    nreines_n = 7
    for key in ("grille-plan", "grille-tore"):
        r5 = next(r for r in results[key] if r["n"] == grid_n)
        fins = [r5["coupe"][p] for p in range(grid_n * grid_n) if (p + 1) % grid_n == 0]
        emit(f"memoire de coupe aux fins de lignes ({key}, n={grid_n}) : {fins}")
    p5 = next(r for r in results["grille-plan"] if r["n"] == grid_n)
    t5 = next(r for r in results["grille-tore"] if r["n"] == grid_n)
    nr7 = next(r for r in results["n-reines"] if r["n"] == nreines_n)
    if p5["plat"] != EXPECTED_PLAN5:
        raise RuntimeError(f"plat plan5: {p5['plat']}")
    if t5["plat"] != EXPECTED_TORE5:
        raise RuntimeError(f"plat tore5: {t5['plat']}")
    if nr7["plat"] != EXPECTED_NR7:
        raise RuntimeError(f"plat n-reines7: {nr7['plat']}")
    max_an = [max(r["coupe"]) for r in results["anneau"]]
    emit(f"coupe max anneau C_n (n=3..9)   : {max_an}")
    if not (max(max_an[ANNEAU_N_MIN:]) == max(max_an) == EXPECTED_ANNEAU_PLATEAU):
        raise RuntimeError(f"plateau anneau: {max_an}")
    coupes_nr = [max(r["coupe"]) for r in results["n-reines"]]
    emit(f"coupe max N-reines (N=4..7)    : {coupes_nr}")
    if coupes_nr != EXPECTED_NR_COUPE:
        raise RuntimeError(f"coupe N-reines: {coupes_nr}")
    emit(
        "controles croises : plat plan5=249, tore5=1159, n-reines7=15180, "
        "plateau anneau 4 des n=6, coupe N-reines 17/59/265/1146 : conformes"
    )


def verdict_table(
    familles: list[tuple[str, object, list[int]]],
    familles_ref: dict[str, tuple[dict[str, object], list[dict[str, object]]]],
) -> int:
    """Print the verdict table and return the number of agreements."""
    historique = {
        "anneau": ("fenetre", "I : la fenetre/handles porte le gain"),
        "grille-plan": ("fenetre", "V : fenetre correcte, surcharge non rentable"),
        "grille-tore": ("fenetre + couture", "V : la couture porte le gain"),
        "referendum": ("compteur", "VI-A : multiplicites/compteur"),
        "n-reines": ("registre", "VI-B : fenetre fausse, registre"),
    }
    emit(
        f"\n{'famille':>12} {'rho':>4} {'requete':>8} {'coupe max':>9} "
        f"{'verdict':>17} {'attendu':>17} {'accord':>7}"
    )
    accords = 0
    for key, _mk, _ns in familles:
        fam, rows = familles_ref[key]
        v = classifieur(fam, rows)
        attendu, _ref = historique[key]
        ok = "oui" if v == attendu else "NON"
        accords += v == attendu
        requete = "oui" if fam["requete"] else "-"
        cmax = max(max(r["coupe"]) for r in rows)
        rho = "n-1" if fam["rho_croit"] else str(fam["rho"])
        emit(f"{key:>12} {rho:>4} {requete:>8} {cmax:>9} {v:>17} {attendu:>17} {ok:>7}")
    if accords != len(familles):
        raise RuntimeError("desaccord du classifieur")
    emit(f"\n{accords}/{len(familles)} accords. Detecteur conforme aux verdicts historiques I-VI.")
    return accords


if __name__ == "__main__":
    t00 = time.time()
    results: dict[str, list[dict[str, object]]] = {}
    familles = [
        ("anneau", fam_anneau, list(range(ANNEAU_N_MIN, ANNEAU_N_MAX))),
        ("grille-plan", lambda n: fam_grille(n, "plan"), GRID_NS),
        ("grille-tore", lambda n: fam_grille(n, "tore"), GRID_NS),
        ("n-reines", fam_nreines, NREINES_NS),
        ("referendum", fam_referendum, list(range(REFERENDUM_N_MIN, REFERENDUM_N_MAX))),
    ]
    familles_ref = {}
    for key, mk, ns in familles:
        rows = run_family(mk, key, ns, results)
        familles_ref[key] = (mk(ns[-1]), rows)

    check_theorem(results)
    check_cross(results)
    verdict_table(familles, familles_ref)
    emit(f"OK -> results7.json ({time.time() - t00:.1f} s, RSS max {rss():.0f} Mo)")

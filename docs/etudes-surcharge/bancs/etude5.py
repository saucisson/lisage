#!/usr/bin/env python3
"""Etude "surcharge" partie V - grilles 2D : coutures croisees et coin.

Encodage bytes, execution par variante, resultats sauvegardes par n.
Modele : langage des ensembles independants d'une grille n*n (cellule {0,1},
garde locale : ajouter un 1 exige les 4 voisins a 0 ; retrait libre).
Variantes de frontiere : tore (2 coutures croisees + coin), cylindre (1
couture), plan (bords libres), tore-presse (cellule (0,0) a alphabet {0,1,2}).

Usage : python3 etude5.py <tore|cylindre|plan|tore-presse> <n_max>
"""

import json
import resource
import sys
import time
from collections import deque
from pathlib import Path

HERE = Path(__file__).resolve().parent
RESULTS = HERE / "results5.json"
WIDTH_CAP = 2
MIN_N = 2
MAX_INVARIANT_CHECK_N = 4


def dims(n: int, wrap: str) -> tuple[list[int], list[int]]:
    """Index ranges of the two axes for the given boundary variant."""
    dis = list(range(n)) if wrap in ("tore", "cylindre") else [0]
    djs = list(range(n)) if wrap == "tore" else [0]
    return dis, djs


def neighbors(i: int, j: int, n: int, wrap: str) -> list[tuple[int, int]]:
    """4-neighbourhood of cell (i, j) under the given boundary variant."""
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


def valid_cols(n: int) -> list[int]:
    """Configurations of a wrapped vertical column (cycle C_n)."""
    out = []
    for b in range(1 << n):
        ok = all(not ((b >> i) & 1 and (b >> ((i + 1) % n)) & 1) for i in range(n))
        if ok:
            out.append(b)
    return out


def precheck(n: int) -> tuple[int, int]:
    """Exact |S| by transfer matrix: tore (trace) and cylindre (chain)."""
    cols = valid_cols(n)
    k = len(cols)
    matrix = [[0] * k for _ in range(k)]
    for a in range(k):
        for b in range(k):
            if cols[a] & cols[b] == 0:
                matrix[a][b] = 1
    # cylindre: n columns in a chain (axis j free)
    v = [1] * k
    for _ in range(n - 1):
        v = [sum(matrix[i][j] * v[j] for j in range(k)) for i in range(k)]
    cyl = sum(v)
    # tore: the columns form a cycle (trace of A^n)
    m = [row[:] for row in matrix]
    for _ in range(n - 1):
        m = [[sum(m[i][t] * matrix[t][j] for t in range(k)) for j in range(k)] for i in range(k)]
    tor = sum(m[i][i] for i in range(k))
    return tor, cyl


def succ_gen(n: int, wrap: str, irregular: bool) -> type:
    """Successor generator with precomputed neighbourhoods."""
    nbs = [[neighbors(i, j, n, wrap) for j in range(n)] for i in range(n)]

    def f(cfg: bytes) -> list[bytes]:
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


def invariants(cfg: bytes, n: int, wrap: str, irregular: bool = False) -> bool:
    """Local guard: no two neighbouring cells are both set."""
    nbs = [[neighbors(i, j, n, wrap) for j in range(n)] for i in range(n)]
    for i in range(n):
        for j in range(n):
            if cfg[i * n + j] >= 1:
                for x, y in nbs[i][j]:
                    if cfg[x * n + y]:
                        return False
    return True


def bfs(init: bytes, succ: type) -> set[bytes]:
    """Reachable configurations from init under succ."""
    seen = {init}
    todo = deque([init])
    while todo:
        st = todo.popleft()
        for t in succ(st):
            if t not in seen:
                seen.add(t)
                todo.append(t)
    return seen


def mdd(vars_n: int, states: set[bytes]) -> int:
    """Number of nodes of the exact positional MDD over the given states."""
    ids = {}

    def build(pos: int, sset: set[bytes]) -> int | str:
        if pos == vars_n:
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
        return r

    build(0, states)
    return len(ids)


def translations(cfg: bytes, n: int, wrap: str) -> type:
    """All translates of cfg under the boundary variant."""
    dis, djs = dims(n, wrap)
    for di in dis:
        for dj in djs:
            b = bytearray(n * n)
            for i in range(n):
                for j in range(n):
                    b[i * n + j] = cfg[((i + di) % n) * n + ((j + dj) % n)]
            yield bytes(b)


def anchor(states: set[bytes], n: int, wrap: str) -> set[bytes]:
    """Canonical translation anchoring of each state."""
    return {min(translations(s, n, wrap)) for s in states}


def cell_signature(
    s: bytes,
    i: int,
    j: int,
    n: int,
    wrap: str,
) -> tuple[int, int, int, int, int, int, int, int, int, int]:
    """Signature of cell (i, j).

    Capped distances to the 4 borders, anchor letter, own value, and the
    4 neighbour values (-1 outside the grid).
    """
    base = i * n
    cx, c_x = min(i, WIDTH_CAP), min(n - 1 - i, WIDTH_CAP)
    cy, c_y = min(j, WIDTH_CAP), min(n - 1 - j, WIDTH_CAP)
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
    return (cx, c_x, cy, c_y, s[0], s[base + j], up, dn, lf, rt)


def pool_2d(
    anchored: set[bytes],
    n: int,
    irregular: bool = False,
    wrap: str = "tore",
) -> tuple[int, int, int]:
    """Pool signature = (capped 4 distances, anchor letter, 5-window)."""
    sig_a: set[tuple[int, int, int, int, int, int, int, int, int, int]] = set()
    sig_b: set[tuple[int, int, int, int, int, int, int, int, int, int]] = set()
    for s in anchored:
        for i in range(n):
            for j in range(n):
                sig = cell_signature(s, i, j, n, wrap)
                interior = sig[0] == sig[1] == sig[2] == sig[3] == WIDTH_CAP
                (sig_a if interior else sig_b).add(sig)
    return len(sig_a) + len(sig_b), len(sig_a), len(sig_b)


def controller_plan(wrap: str, irregular: bool) -> list[str]:
    """Controller description lines for the report."""
    p = []
    if wrap == "tore":
        p.append(
            "2 coutures d'enroulement croisees + coin : ancre translation, fenetres plafonnees"
        )
    elif wrap == "cylindre":
        p.append("1 couture d'enroulement (axe i) : ancre 1D.")
    else:
        p.append("bords libres : aucune couture.")
    if irregular:
        p.append(
            "module singleton (cellule {0,1,2}) : couture de classe, ancre = cellule irreguliere."
        )
    return p


def load_results() -> dict[str, list[dict[str, object]]]:
    """Load previously saved results, empty on any read/parse failure."""
    try:
        with RESULTS.open(encoding="utf-8") as f:
            data: dict[str, list[dict[str, object]]] = json.load(f)
            return data
    except OSError:
        return {}
    except json.JSONDecodeError:
        return {}


def save_results(results: dict[str, list[dict[str, object]]]) -> None:
    """Persist results next to this script."""
    with RESULTS.open("w", encoding="utf-8") as f:
        json.dump(results, f, indent=1)


def emit(line: str) -> None:
    """Write one line to stdout (CLI entry point of the bench)."""
    sys.stdout.write(line + "\n")
    sys.stdout.flush()


def run(wrap: str, irregular: bool, n_max: int) -> list[dict[str, object]]:
    """Run the bench for n in [MIN_N, n_max], printing and saving each row."""
    rows: list[dict[str, object]] = []
    results = load_results()
    key = "tore-presse" if irregular else wrap
    for n in range(MIN_N, n_max + 1):
        t0 = time.time()
        init = bytes(n * n)
        succ = succ_gen(n, wrap, irregular)
        states = bfs(init, succ)
        if wrap == "tore" and not irregular:
            expected = precheck(n)[0]
            if len(states) != expected:
                raise RuntimeError(f"tore precheck mismatch: |S|={len(states)} expected={expected}")
        if n <= MAX_INVARIANT_CHECK_N:
            for c in states:
                if not invariants(c, n, wrap, irregular):
                    raise RuntimeError("invariant viole")
        plat = mdd(n * n, states)
        anch = states if irregular else anchor(states, n, wrap)
        tot, a_in, b_bd = pool_2d(anch, n, irregular, wrap=wrap)
        row: dict[str, object] = {
            "n": n,
            "states": len(states),
            "anchored": len(anch),
            "S_div_n2": len(anch) / (n * n),
            "plat": plat,
            "pool": tot,
            "A_int": a_in,
            "B_bord": b_bd,
            "sec": round(time.time() - t0, 1),
        }
        rows.append(row)
        emit(json.dumps(row))
        results[key] = rows
        save_results(results)
    return rows


def show(name: str, rows: list[dict[str, object]]) -> None:
    """Pretty-print the result table."""
    emit(f"=== {name} ===")
    emit(
        f"{'n':>2} {'|S|':>9} {'ancre':>9} {'/n2':>5} {'plat':>8} "
        f"{'pool':>5} {'A(int)':>6} {'B(bord)':>7}"
    )
    for r in rows:
        emit(
            f"{r['n']:>2} {r['states']:>9} {r['anchored']:>9} {r['S_div_n2']:>5.1f} "
            f"{r['plat']:>8} {r['pool']:>5} {r['A_int']:>6} {r['B_bord']:>7}"
        )
    if len(rows) > 1:
        plat_growth = [
            round(rows[i + 1]["plat"] / rows[i]["plat"], 1) for i in range(len(rows) - 1)
        ]
        emit(f"ratios plat successifs : {plat_growth}")
    emit(f"pool successifs        : {[r['pool'] for r in rows]}")
    emit("")


if __name__ == "__main__":
    w = sys.argv[1]
    nmax = int(sys.argv[2])
    irr = w == "tore-presse"
    base = "tore" if w.startswith("tore") else w
    tor, cyl = precheck(nmax)
    emit(f"[pre-check n={nmax} : tore={tor}, cylindre={cyl}]")
    rows = run(base, irr, nmax)
    show(w, rows)
    rss_mo = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024.0
    emit(f"RSS max : {rss_mo:.0f} Mo")

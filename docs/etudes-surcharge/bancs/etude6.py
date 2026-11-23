#!/usr/bin/env python3
"""Etude "surcharge" partie VI - la memoire : exigible, mais fragile.

Deux proxys opposes du catalogue MCC (Referendum, NQueens) ou la pathologie
n'est ni la couture ni le span de garde, mais l'information qui n'est pas
dans la fenetre. A. Referendum : dynamique libre, requete globale (compteur
de oui majoritaire). B. N-reines : garde filtrante globale (portee N).

Usage : python3 etude6.py <referendum|nqueens|portees> <n_max>
"""

import json
import resource
import sys
import time
from collections import deque
from pathlib import Path

HERE = Path(__file__).resolve().parent
RESULTS = HERE / "results6.json"
W = 2
MIN_REFERENDUM_N = 2
MIN_NQUEENS_N = 4
PCT = 100.0
SOLUTIONS = {1: 1, 2: 0, 3: 0, 4: 2, 5: 10, 6: 4, 7: 40, 8: 92}


def rss() -> float:
    """Peak RSS of this process, in MiB."""
    return resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024.0


def emit(line: str) -> None:
    """Write one line to stdout (CLI entry point of the bench)."""
    sys.stdout.write(line + "\n")
    sys.stdout.flush()


def save(key: str, rows: object) -> None:
    """Merge rows under key into the results file next to this script."""
    try:
        with RESULTS.open(encoding="utf-8") as f:
            d: dict[str, object] = json.load(f)
    except OSError:
        d = {}
    except json.JSONDecodeError:
        d = {}
    d[key] = rows
    with RESULTS.open("w", encoding="utf-8") as f:
        json.dump(d, f, indent=1)


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


def mdd(states: set[bytes]) -> int:
    """Exact positional MDD: each node is one suffix class."""
    nvars = len(next(iter(states)))
    ids: dict[object, int] = {}

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
        return r

    build(0, states)
    return len(ids)


# ----------------------------- A. referendum -----------------------------
def verdict(v: bytes) -> int:
    """1 if the yes votes are a strict majority (everyone voted), else 2."""
    return 1 if v.count(1) > v.count(2) else 2


def succ_scrutin(n_voters: int) -> object:
    """Successor generator of the referendum dynamics (one extra verdict var)."""

    def f(s: bytes) -> list[bytes]:
        out = []
        if s[n_voters] == 0:
            for i in range(n_voters):
                if s[i] == 0:
                    for x in (1, 2):
                        c = bytearray(s)
                        c[i] = x
                        out.append(bytes(c))
            if all(s[i] for i in range(n_voters)):
                c = bytearray(s)
                c[n_voters] = verdict(s)
                out.append(bytes(c))
        return out

    return f


Triplet = tuple[int, int, int]


def local_classes(v: bytes, n: int) -> tuple[tuple[Triplet, ...], frozenset[Triplet]]:
    """Local types of a vote ring: multiset vs set of neighbour triplets."""
    trips = [(v[(i - 1) % n], v[i], v[(i + 1) % n]) for i in range(n)]
    return tuple(sorted(trips)), frozenset(trips)


def class_stats(states: set[bytes], n: int) -> tuple[int, int, int, int, int]:
    """Pool sizes (multiset, set) and ambiguity counters of the local types."""
    classes_ms: dict[object, set[int]] = {}
    classes_set: dict[object, set[tuple[int, bool]]] = {}
    for v in states:
        ms, ss = local_classes(v, n)
        o, nn = v.count(1), v.count(2)
        classes_ms.setdefault(ms, set()).add(o)
        classes_set.setdefault(ss, set()).add((o, nn + (n - o - nn) >= o))
    for m in classes_ms.values():
        if len(m) != 1:
            raise RuntimeError("le multiset doit donner le compte exact (marge centrale)")
    amb = dec_amb = k_max = 0
    for members in classes_set.values():
        counts = {c[0] for c in members}
        if len(counts) > 1:
            amb += 1
            k_max = max(k_max, len(counts))
            # ambiguous decision: the class mixes a "certainly adopted"
            # member (reversal impossible) with a reversible one
            if len({c[1] for c in members}) > 1:
                dec_amb += 1
    return len(classes_ms), len(classes_set), amb, dec_amb, k_max


def run_referendum(n_max: int) -> None:
    """Run the referendum bench for N in [MIN_REFERENDUM_N, n_max]."""
    rows: list[dict[str, object]] = []
    for n in range(MIN_REFERENDUM_N, n_max + 1):
        t0 = time.time()
        s2 = bfs(bytes(n + 1), succ_scrutin(n))
        states = {s[:n] for s in s2 if s[n] == 0}
        if len(states) != 3**n:
            raise RuntimeError(f"referendum |S| mismatch: {n}, {len(states)}")
        if len(s2) != 3**n + 2**n:
            raise RuntimeError(f"referendum |S2| mismatch: {n}, {len(s2)}")
        plat = mdd(states)
        plat_req = mdd(s2)
        compteur = (n + 1) * (n + 2) // 2
        # local types: multiset (correct) vs set (loses multiplicities)
        pool_ms, pool_set, amb, dec_amb, k_max = class_stats(states, n)
        row: dict[str, object] = {
            "N": n,
            "states": 3**n,
            "plat": plat,
            "plat_req": plat_req,
            "compteur": compteur,
            "pool_ms": pool_ms,
            "pool_set": pool_set,
            "amb": amb,
            "dec_amb": dec_amb,
            "k_max": k_max,
            "sec": round(time.time() - t0, 1),
        }
        rows.append(row)
        emit(json.dumps(row))
        save("referendum", rows)
    emit(
        f"{'N':>2} {'|S|':>7} {'plat':>4} {'+req':>4} {'cpt':>4} "
        f"{'ms':>6} {'set':>6} {'amb':>4} {'dec':>4} {'k':>2}"
    )
    for r in rows:
        emit(
            f"{r['N']:>2} {r['states']:>7} {r['plat']:>4} {r['plat_req']:>4} "
            f"{r['compteur']:>4} {r['pool_ms']:>6} {r['pool_set']:>6} "
            f"{r['amb']:>4} {r['dec_amb']:>4} {r['k_max']:>2}"
        )


# ----------------------------- B. N-queens -----------------------------
def conflits(n: int) -> list[frozenset[int]]:
    """Conflicting cells (row, column, both diagonals) for each cell p."""
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


def voisins8(n: int) -> list[tuple[int, ...]]:
    """Moore neighbourhood (8 neighbours) of each cell."""
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


def succ_reines(n: int, confl: list[frozenset[int]]) -> object:
    """Successor generator of the N-queens dynamics."""

    def f(s: bytes) -> list[bytes]:
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


Register = tuple[frozenset[int], frozenset[int], frozenset[int], frozenset[int]]


def empreinte(s: bytes, n: int) -> Register:
    """Occupation register: rows, columns, both diagonals."""
    rows, cols, d1, d2 = [], [], [], []
    for p in range(n * n):
        if s[p]:
            i, j = divmod(p, n)
            rows.append(i)
            cols.append(j)
            d1.append(i - j)
            d2.append(i + j)
    return (frozenset(rows), frozenset(cols), frozenset(d1), frozenset(d2))


def window_signatures(states: set[bytes], n: int, v8: list[tuple[int, ...]]) -> set[object]:
    """Bounded-window signatures.

    Capped distances to the 4 borders, own cell, and the 8 Moore neighbours.
    """
    sigs = set()
    for s in states:
        for p in range(n * n):
            i, j = divmod(p, n)
            sigs.add(
                (
                    min(i, W),
                    min(n - 1 - i, W),
                    min(j, W),
                    min(n - 1 - j, W),
                    s[p],
                    tuple(s[q] for q in v8[p]),
                )
            )
    return sigs


def false_positives(
    states: set[bytes],
    n: int,
    confl: list[frozenset[int]],
    win: list[tuple[int, ...]],
) -> tuple[int, int]:
    """Count (false positives, empty-cell probes).

    The window believes an addition is legal while the semantics refuses it.
    """
    fp = e = 0
    for s in states:
        for p in range(n * n):
            if s[p]:
                continue
            e += 1
            if not any(s[q] for q in win[p]) and any(s[q] for q in confl[p]):
                fp += 1
    return fp, e


def run_nqueens(n_max: int) -> None:
    """Run the N-queens bench for n in [MIN_NQUEENS_N, n_max]."""
    rows: list[dict[str, object]] = []
    for n in range(MIN_NQUEENS_N, n_max + 1):
        t0 = time.time()
        confl = conflits(n)
        v8 = voisins8(n)
        states = bfs(bytes(n * n), succ_reines(n, confl))
        pleines = sum(1 for s in states if sum(s) == n)
        if pleines != SOLUTIONS[n]:
            raise RuntimeError(f"nqueens solutions mismatch: {n}, {pleines}")
        plat_ligne = mdd(states)
        # anti-diagonal sweep order
        perm = sorted(range(n * n), key=lambda p: (p // n + p % n, p % n))
        sd = {bytes(s[perm[q]] for q in range(n * n)) for s in states}
        plat_diag = mdd(sd)
        sigs = window_signatures(states, n, v8)
        fp, e = false_positives(states, n, confl, v8)
        emp = {empreinte(s, n) for s in states}
        row: dict[str, object] = {
            "n": n,
            "states": len(states),
            "pleines": pleines,
            "plat_ligne": plat_ligne,
            "plat_diag": plat_diag,
            "pool": len(sigs),
            "err_pct": round(PCT * fp / e, 2),
            "empreintes": len(emp),
            "sec": round(time.time() - t0, 1),
        }
        rows.append(row)
        emit(json.dumps(row))
        save("nqueens", rows)
    emit(
        f"{'N':>2} {'|S|':>7} {'pl.':>3} {'ligne':>7} {'diag':>7} "
        f"{'pool':>5} {'err%':>6} {'emp.':>6}"
    )
    for r in rows:
        emit(
            f"{r['n']:>2} {r['states']:>7} {r['pleines']:>3} {r['plat_ligne']:>7} "
            f"{r['plat_diag']:>7} {r['pool']:>5} {r['err_pct']:>6} {r['empreintes']:>6}"
        )


def fenetre_portees(n: int) -> dict[int, float]:
    """False-positive rate curve of a Chebyshev window of radius R."""
    confl = conflits(n)
    states = bfs(bytes(n * n), succ_reines(n, confl))
    out: dict[int, float] = {}
    for radius in range(1, n):
        win = []
        for p in range(n * n):
            i, j = divmod(p, n)
            w = []
            for di in range(-radius, radius + 1):
                for dj in range(-radius, radius + 1):
                    a, b = i + di, j + dj
                    if 0 <= a < n and 0 <= b < n:
                        w.append(a * n + b)
            win.append(tuple(w))
        fp, e = false_positives(states, n, confl, win)
        out[radius] = round(PCT * fp / e, 2)
    emit(json.dumps({"N": n, "portees": out}))
    save(f"portees-{n}", {"N": n, "portees": {str(k): v for k, v in out.items()}})
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
    emit(f"RSS max : {rss():.0f} Mo")

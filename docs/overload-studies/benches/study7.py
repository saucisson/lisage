#!/usr/bin/env python3
"""Overload study part VII - detect before reducing.

From static reads of the guards alone (supports R(q), without enumerating
the language), recover the manual verdicts of parts I-VI and predict the
flat MDD. Three readings: model radius rho, query shape, cut memory I(p).

Theorem of the bench: the number of flat-MDD nodes at level p equals the
number of distinct I(p) motifs over real prefixes.

Usage: python3 study7.py
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
THRESHOLD = 2_000_000
MIN_GRID_N = 3
MIN_SQUARE_CHECK = 3
SEAM_FACTOR = 2
RING_N_MIN = 3
RING_N_MAX = 10
GRID_NS = [2, 3, 4, 5]
NQUEENS_NS = [4, 5, 6, 7]
REFERENDUM_N_MIN = 3
REFERENDUM_N_MAX = 11
EXPECTED_PLANE5 = 249
EXPECTED_TORUS5 = 1159
EXPECTED_NQ7 = 15180
EXPECTED_RING_PLATEAU = 4
EXPECTED_NQ_CUT = [17, 59, 265, 1146]


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


def cut_memory(states: set[bytes], reads: list[frozenset[int]]) -> list[int]:
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


def cell_projections(
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
def fam_ring(n: int) -> dict[str, object]:
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
        "query": False,
        "rho": 1,
        "rho_grows": False,
        "name": f"C_{n}",
    }


def fam_grid(n: int, wrap: str) -> dict[str, object]:
    """Independent sets of an n*n grid (part V), row by row."""
    reads = []
    for i in range(n):
        for j in range(n):
            out = set()
            for di, dj in ((-1, 0), (1, 0), (0, -1), (0, 1)):
                a, b = i + di, j + dj
                if wrap == "torus":
                    out.add((a % n) * n + (b % n))
                elif 0 <= a < n and 0 <= b < n:
                    out.add(a * n + b)
            out.discard(i * n + j)
            reads.append(frozenset(out))
    neighbors = [list(r) for r in reads]

    def succ(s: bytes) -> list[bytes]:
        out = []
        for p in range(n * n):
            if s[p]:
                c = bytearray(s)
                c[p] = 0
                out.append(bytes(c))
            elif not any(s[q] for q in neighbors[p]):
                c = bytearray(s)
                c[p] = 1
                out.append(bytes(c))
        return out

    return {
        "S": bfs(bytes(n * n), succ),
        "R": reads,
        "query": False,
        "rho": 1,
        "rho_grows": False,
        "name": f"grid-{wrap} {n}x{n}",
    }


def fam_nqueens(size: int) -> dict[str, object]:
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
        "query": False,
        "rho": size - 1,
        "rho_grows": True,
        "name": f"N-queens {size}x{size}",
    }


def fam_referendum(n_voters: int) -> dict[str, object]:
    """Free N-voter ballot (part VI): free guard, majority query."""
    states = {bytes(c) for c in itertools.product((0, 1, 2), repeat=n_voters)}
    return {
        "S": states,
        "R": [frozenset()] * n_voters,
        "query": True,
        "rho": 0,
        "rho_grows": False,
        "name": f"referendum N={n_voters}",
    }


# ----------------------------- classifier -----------------------------
def classifier(fam: dict[str, object], rows: list[dict[str, object]]) -> str:
    """(radius, query, cut profile) -> rung verdict."""
    if fam["query"]:
        return "counter"
    if fam["rho_grows"]:
        return "register"
    # seam bulge: at the row ends of the largest square instance, inner peak
    # >= 2 x inner trough (static seam)
    ends_all = []
    for r in rows:
        m = round(math.sqrt(len(r["cut"])))
        if m * m == len(r["cut"]) and m >= MIN_SQUARE_CHECK:
            ends_all = [r["cut"][p] for p in range(m * m) if (p + 1) % m == 0]
    if ends_all:
        interior = ends_all[1:-1]
        if interior and max(interior) >= SEAM_FACTOR * min(interior):
            return "window + seam"
    return "window"


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
        if len(states) * nvars > THRESHOLD:
            raise RuntimeError(f"threshold exceeded: {key}, {n}, {len(states)}, {nvars}")
        flat = mdd_levels(states)
        cut = cut_memory(states, fam["R"])
        proj = cell_projections(states, fam["R"], nvars)
        equal = sum(1 for p in range(nvars) if flat[p] == cut[p])
        equal_proj = sum(1 for p in range(nvars) if flat[p] == proj[p])
        rows.append(
            {
                "n": n,
                "states": len(states),
                "levels": nvars,
                "flat": sum(flat),
                "cut": cut,
                "cut_max": max(cut),
                "equality": f"{equal}/{nvars}",
                "equal_proj": f"{equal_proj}/{nvars}",
                "sec": round(time.time() - t0, 1),
            }
        )
        emit(
            json.dumps(
                {
                    "family": key,
                    "n": n,
                    "states": len(states),
                    "flat": sum(flat),
                    "cut_max": max(cut),
                    "equality": f"{equal}/{nvars}",
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
    total_levels = sum(r["levels"] for rows in results.values() for r in rows)
    total_equal = sum(int(r["equality"].split("/")[0]) for rows in results.values() for r in rows)
    total_proj = sum(int(r["equal_proj"].split("/")[0]) for rows in results.values() for r in rows)
    emit(f"\ntheorem equality (nodes = |I(p)|) : {total_equal}/{total_levels} levels")
    emit(f"counter-hypothesis (projections) : {total_proj}/{total_levels} levels")
    if total_equal != total_levels:
        raise RuntimeError("theorem violated")
    return total_equal, total_levels, total_proj


def check_cross(results: dict[str, list[dict[str, object]]]) -> None:
    """Seam bulge and cross-checks against parts V and VI."""
    grid_n = 5
    nqueens_n = 7
    for key in ("grid-plane", "grid-torus"):
        r5 = next(r for r in results[key] if r["n"] == grid_n)
        ends = [r5["cut"][p] for p in range(grid_n * grid_n) if (p + 1) % grid_n == 0]
        emit(f"cut memory at row ends ({key}, n={grid_n}) : {ends}")
    p5 = next(r for r in results["grid-plane"] if r["n"] == grid_n)
    t5 = next(r for r in results["grid-torus"] if r["n"] == grid_n)
    nq7 = next(r for r in results["n-queens"] if r["n"] == nqueens_n)
    if p5["flat"] != EXPECTED_PLANE5:
        raise RuntimeError(f"flat plane5: {p5['flat']}")
    if t5["flat"] != EXPECTED_TORUS5:
        raise RuntimeError(f"flat torus5: {t5['flat']}")
    if nq7["flat"] != EXPECTED_NQ7:
        raise RuntimeError(f"flat n-queens7: {nq7['flat']}")
    max_ring = [max(r["cut"]) for r in results["ring"]]
    emit(f"max cut ring C_n (n=3..9)   : {max_ring}")
    if not (max(max_ring[RING_N_MIN:]) == max(max_ring) == EXPECTED_RING_PLATEAU):
        raise RuntimeError(f"ring plateau: {max_ring}")
    cuts_nq = [max(r["cut"]) for r in results["n-queens"]]
    emit(f"max cut N-queens (N=4..7)    : {cuts_nq}")
    if cuts_nq != EXPECTED_NQ_CUT:
        raise RuntimeError(f"n-queens cut: {cuts_nq}")
    emit(
        "cross-checks: flat plane5=249, torus5=1159, n-queens7=15180, "
        "ring plateau 4 from n=6, n-queens cut 17/59/265/1146 : consistent"
    )


def verdict_table(
    families: list[tuple[str, object, list[int]]],
    families_ref: dict[str, tuple[dict[str, object], list[dict[str, object]]]],
) -> int:
    """Print the verdict table and return the number of agreements."""
    historical = {
        "ring": ("window", "I: the window/handles carry the gain"),
        "grid-plane": ("window", "V: window correct, overload not profitable"),
        "grid-torus": ("window + seam", "V: the seam carries the gain"),
        "referendum": ("counter", "VI-A: multiplicities/counter"),
        "n-queens": ("register", "VI-B: window wrong, register"),
    }
    emit(
        f"\n{'family':>12} {'rho':>4} {'query':>8} {'max cut':>9} "
        f"{'verdict':>17} {'expected':>17} {'match':>7}"
    )
    agreements = 0
    for key, _mk, _ns in families:
        fam, rows = families_ref[key]
        v = classifier(fam, rows)
        expected, _ref = historical[key]
        ok = "yes" if v == expected else "NO"
        agreements += v == expected
        query = "yes" if fam["query"] else "-"
        cmax = max(max(r["cut"]) for r in rows)
        rho = "n-1" if fam["rho_grows"] else str(fam["rho"])
        emit(f"{key:>12} {rho:>4} {query:>8} {cmax:>9} {v:>17} {expected:>17} {ok:>7}")
    if agreements != len(families):
        raise RuntimeError("classifier disagreement")
    emit(
        f"\n{agreements}/{len(families)} agreements. "
        "Detector consistent with the historical verdicts I-VI."
    )
    return agreements


if __name__ == "__main__":
    t00 = time.time()
    results: dict[str, list[dict[str, object]]] = {}
    families = [
        ("ring", fam_ring, list(range(RING_N_MIN, RING_N_MAX))),
        ("grid-plane", lambda n: fam_grid(n, "plane"), GRID_NS),
        ("grid-torus", lambda n: fam_grid(n, "torus"), GRID_NS),
        ("n-queens", fam_nqueens, NQUEENS_NS),
        ("referendum", fam_referendum, list(range(REFERENDUM_N_MIN, REFERENDUM_N_MAX))),
    ]
    families_ref = {}
    for key, mk, ns in families:
        rows = run_family(mk, key, ns, results)
        families_ref[key] = (mk(ns[-1]), rows)

    check_theorem(results)
    check_cross(results)
    verdict_table(families, families_ref)
    emit(f"OK -> results7.json ({time.time() - t00:.1f} s, max RSS {rss():.0f} MiB)")

---
name: "study-overload-protocols"
title: "Study \u201coverload\u201d II: less regular protocols (Peterson & co)"
type: "text/markdown"
---

# Study on "homogeneity overload" — part II: less regular protocols

Follow-up of part I (rings: Philo, Presser, Token). Same machinery (flat =
exact positional MDD, anchored = exact MDD over the rotation-anchored set,
window = bounded-distance quotient), applied this time to the "tier-2"
families: mutual exclusion through shared memory.

## 1. Families

| Family | What | Asymmetry |
|---|---|---|
| **pet2** | Exact Peterson, 2 processes (flags + turn) | frozen n: P0/P1 swap + turn flip |
| **dek2** | Exact Dekker, 2 processes | same |
| **petN** | Generalized Peterson n processes = "filter" lock: n−1 levels, shared victim array V[ℓ], ID-dependent guards (V[ℓ] == i) | the shared array + circulation of the "last arrived": the tier-2 of the debate (Anderson, Eisenberg-McGuire) |
| **petD** | petN + private chain of length i+1 per process | **diffuse irregularity**: n singleton classes, negative control |

Validations: mutual exclusion checked over all reached states (never
violated); for petN, the rotational symmetry is **exact**: |S| = n ·
|anchored-without-duplicate| at all n (5/10, 27/81, 219/876, 2273/11365,
28123/168738), no degenerate orbit.

## 2. Frozen instances (pet2, dek2)

| Model | \|S\| | flat |
|---|---|---|
| pet2 | 21 | 21 |
| dek2 | 25 | 20 |

Reading: at constant n = 2, there is nothing to fold. The swap quotient buys
at best a factor 2 on states. These instances serve as guard rails (mutual
exclusion is checked there), not as curves.

## 3. Generalized Peterson (petN): the strong result

| n | \|S\| | flat | anchored (exact) | window pool | forms |
|---|---|---|---|---|---|
| 2 | 10 | 7 | 5 | 10 | 3 |
| 3 | 81 | 48 | 25 | 42 | 4 |
| 4 | 876 | 328 | 117 | 152 | 5 |
| 5 | 11 365 | 2 753 | 739 | 415 | 6 |
| 6 | 168 738 | 27 462 | 5 949 | 556 | 7 |

Three clear teachings:

1. **The flat explodes.** Fictitious slope 5 761 nodes/n: this is no longer
   linear, it is super-linear. At n = 6 the exact MDD keeps 27 462 nodes
   for 168 738 states (ratio 16 %): suffix sharing barely compresses
   anything any more as soon as the guards quantify over **all** the
   processes (no more locality). Even the exact anchored collapses (5 949):
   anchoring without a window saves no more here than on the rings.
2. **The quotient pays immediately.** Crossing point between n = 3 (48
   against 42): where the rings of part I required n ≈ 30, the protocols
   pay from toy sizes on. And the gap becomes brutal: ×6.6 in nodes at
   n = 5, **×49 at n = 6**, and it widens super-exponentially (the flat
   follows |S| closely, the pool follows a polynomial law).
3. **But the pool no longer plateaus.** The O(1) plateau of part I is gone:
   10 → 556, roughly quadratic growth. The reason is structural, not a
   glitch: the number of **levels** of the filter grows with n (n−1
   victims, forms = n+1). The scaling parameter of petN is **structural**,
   not topological: the alphabet itself is a function of n.

## 4. Negative control (petD): diffuse irregularity

| n | \|S\| | flat | window pool |
|---|---|---|---|
| 2 | 14 | 18 | 14 |
| 3 | 166 | 129 | 48 |
| 4 | 2 512 | 1 042 | 120 |
| 5 | 44 248 | 10 052 | 245 |

Honest nuance: even the diffuse one sees the pool grow far slower than the
flat (245 against 10 052 at n = 5). But the class count is here
**optimistic**: without rotational symmetry, the windowed representation
does not re-deploy into an exact engine without decorating every position
(which the classifier refuses, verdict "flat"). The apparent gain of the
diffuse one is not cashable; that of petN is (exact symmetry checked
above).

## 5. Classifier verdicts (before construction)

| Family | Static diagnosis | Rendered plan |
|---|---|---|
| pet2, dek2 | 2 modules, 1 class up to renaming (swap + turn flip) | swap anchor: factor 2, nothing else |
| petN | 1 class up to rotation (the guards reference the ID only through V[ℓ] == i); the holder of the highest level circulates | **dynamic anchor** + **shared-level overload** (victim array), no positional duplication |
| petD | n singleton classes (private chains of distinct lengths) | **assumed flat, no injection**: decorating would cost more than it buys |

To be linked with part I: the three plans were again rendered before any
BFS, and the diagnosis of petN ("decorate the level of the shared object,
let the rest fold") is exactly the level overload under discussion: here it
does not replace a constant plateau, it holds the pool polynomial while
the flat goes super-linear.

## 6. Cross synthesis with part I

| | Rings (part I) | Protocols (part II) |
|---|---|---|
| flat | Θ(n), slopes 2 to 5 | **super-linear** (explodes with \|S\|) |
| window pool | O(1) plateau (56 to 183) | polynomial (~n²), no plateau |
| crossing | n ≈ 29 to 46 | **n ≈ 3** |
| alphabets | constant in n | grow with n (structural parameter) |

The table of "very good scores" is thus refined: **O(1) quotient for the
topologically scalable families** (rings, grids: constant alphabet);
**polynomial quotient but crushing the flat for the shared-memory
protocols** — and it is precisely in those families that the
flat-mess-with-sharing engine dies, so that the gap widens fastest.
Peterson & co are less regular, but it is there that the overload has the
most margin.

## 7. Limits and follow-ups

- pool = class count (bounded window W = 2, victim mentions capped at 3
  levels), not a count of `created` by a deployable engine; exactness
  remains to be confirmed in `lisage-bench` (M7), where petN is the
  priority candidate.
- Filter lock modelled with victim writing at arrival only; the variants:
  natural follow-up = Eisenberg-McGuire, Szymanski, and tournament Peterson
  (tree: the pool should drop to O(log n), an interesting in-between).
- petD confirms that detection is the guard rail: injecting into the
  diffuse one would cost non-profitable decorations; keep petN's empty-plan
  and petD's "flat" verdict as classifier non-regression tests, alongside
  the zero-injection of part I.

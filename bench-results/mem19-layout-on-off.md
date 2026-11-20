# MEM-19: layout on/off confrontation (issue #135)

Seeded BDD workload, n = 20000 interned degree-2 nodes, one runner process per layout.

| layout | nodes | bytes/node (end-to-end) | arena bytes/node | temps_ms | rss_max_ko |
|---|---|---|---|---|---|
| on (inline exact classes, MEM-16) | 20002 | 67.32 | 24.00 | 27.124 | 9128 |
| off (catch-all boxed entries) | 20002 | 75.32 | 32.00 | 27.353 | 10904 |

Factors: arena 1.333x, end-to-end 1.119x, RSS 1.195x. Node counts identical (C1).

Verdict: the factor is **constant** in n (1.33x on arena slots, ~1.12x end-to-end,
~1.20x RSS after ALIGN-1/ALIGN-2 shrank both layouts; it was 1.67x/~1.2x/~1.26x on the
pre-ALIGN baseline) — the x5..x12 prediction applies to the cumulative MEM-B track against
the ~420 B/node pre-MEM-B baseline (measured 420/60.75 = 6.9x), not to the layout
         toggle alone.

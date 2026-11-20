# MEM-19: layout on/off confrontation (issue #135)

Seeded BDD workload, n = 20000 interned degree-2 nodes, one runner process per layout.

| layout | nodes | bytes/node (end-to-end) | arena bytes/node | temps_ms | rss_max_ko |
|---|---|---|---|---|---|
| on (inline exact classes, MEM-16) | 20002 | 67.32 | 24.00 | 25.191 | 9128 |
| off (catch-all boxed entries) | 20002 | 83.32 | 40.00 | 25.676 | 10648 |

Factors: arena 1.667x, end-to-end 1.238x, RSS 1.167x. Node counts identical (C1).

Verdict: the factor is **constant** in n (1.67x on arena slots, ~1.2x end-to-end,
~1.26x RSS) — the x5..x12 prediction applies to the cumulative MEM-B track against
the ~420 B/node pre-MEM-B baseline (measured 420/60.75 = 6.9x), not to the layout
         toggle alone.

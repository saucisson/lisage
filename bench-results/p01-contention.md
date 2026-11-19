# P0-1 contention bench — philosophers saturation (issue #136)

Runner `bench_contention {n}`, one process per run. Anchored criterion n = 5;
the saturation step is a synthetic contention step (one fixed illegal member),
exercising rounds/memo/union on the philosophers diagram.

| n | nodes | verified | rounds | creations | shares | R-share hit rate (%) | memo hits | memo misses | memo hit rate | build (ms) | saturation (ms) | count (ms) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 5 | 1860 | 153 | 2 | 1860 | 3510 | 65 | 43 | 158 | 0.2139 | 29.533 | 1.664 | 1.505 |

Interns per level (n = 5): [306, 306, 304, 304, 302, 302, 291, 291, 291, 291, 267, 267, 267, 267, 213, 213, 213, 213, 153, 153]

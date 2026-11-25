# P1-2 merge bench — output-pool fusion (issue #140)
Machine: M1 or M2 (committed during the evening of 2026-11-19, the M1->M2 switch window — see bench-results/machines.md)


Runner `bench_merge {n} {pools}`: 16 output pools, each a full copy of a random
Map diagram (n = 8: 16 keys, domain {0,1,2}, 4000 words, 136286 live nodes),
merged sequentially vs in parallel (pairwise tree reduction, cross-pool sharing).
PAR-1 sanity (same merged cardinality seq vs par) is asserted inside the runner.

| n | pools | input live | merged live | seq (ms) | par (ms) | speedup |
|---|---|---|---|---|---|---|
| 8 | 16 | 136286 | 17928 | 1481.866 | 1091.228 | 1.358 |

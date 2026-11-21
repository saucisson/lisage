# P1-1 parallel ops bench — multi-query workload (issue #139)
Machine: M1 or M2 (committed during the evening of 2026-11-19, the M1->M2 switch window — see bench-results/machines.md)


Runner `bench_parallel_ops {n} {ops}`: 16 independent counting transductions
on roots of the same read-shared input pool (philosophers R_8, 3104 members),
sequential vs rayon parallel (INV10). PAR-1 sanity (same synthesized attributes
seq vs par) is asserted inside the runner.

| n | ops | members | seq (ms) | par (ms) | speedup |
|---|---|---|---|---|---|
| 8 | 16 | 3104 | 23.065 | 10.257 | 2.249 |

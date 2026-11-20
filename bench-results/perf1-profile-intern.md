# PERF-1 profile — alea-lcg intern path (issue #280)
Machine: M1


Runner `profile_intern {n}`, one process per run, counting global allocator (MEM-18 pattern).

| n | noeuds | internements | temps table (ms) | temps build (ms) | temps count (ms) | allocs build | octets build | allocs/internement | rss_max_ko |
|---|---|---|---|---|---|---|---|---|---|
| 20 | 106760 | 1048575 | 1.790 | 583.673 | 41.993 | 8890949 | 468087764 | 8.479 | 15208 |

# Machine register (ALIGN-7b)

Append-only register of the development machines on which
`bench-results/` deposits were measured. One section per machine.

**Rule**: every publication in `bench-results/` carries a `Machine:`
header line (ALIGN-7c). Absolute wall-clock times are only comparable
within one machine; across machines, only ratios are comparable
(ALIGN-7c).

**Machine history** (corrected 2026-11-20 evening, after a human answer
contradicted the first version of this register — the register is
adapted, not forced):

1. **M1** (this machine) was used from the first deposit up to the
   evening of 2026-11-19.
2. Switch to **M2** (a more recent machine, shared with other users)
   during the evening of 2026-11-19: deposits from the night of
   2026-11-19/20 up to the evening of 2026-11-20 are on **M2**
   (align1, committed 2026-11-20 08:04 local, is the M2 deposit).
3. Switch **back to M1** on the evening of 2026-11-20 (M2 is used by
   others): deposits from the evening of 2026-11-20 onward are on
   **M1** again.

Deposits committed during the first switch window (p11, p12 — see
their headers) carry an explicit uncertainty note; the exact switch
hour of 2026-11-19 evening is unknown.

The fingerprint of each machine is emitted by
`make machine-fingerprint` / `python -m scripts machine-fingerprint` (ALIGN-7a); the
`hardware_hash` line below is the key the script matches on.

---

## M1 — this development machine (WSL2)

- **Hardware hash**: `febd14ed`
- **Fingerprint** (emitted by `python -m scripts machine-fingerprint` on
  2026-11-20, captured 2026-11-20T11:56:45Z, WSL2):

```json
{"cpu_model": "Intel(R) Core(TM) i5-3470 CPU @ 3.20GHz", "date_utc": "2026-11-20T11:56:45+00:00", "hardware_hash": "febd14ed", "kernel": "6.18.58.1-microsoft-standard-WSL2", "logical_cores": 4, "machine_id": null, "page_size": 4096, "physical_cores": 4, "ram_total_kib": 3979828, "sources": {"cpu_model": "/proc/cpuinfo: model name", "kernel": "uname -r (platform.release())", "logical_cores": "/proc/cpuinfo: count of 'processor' entries", "machine_id": "no register given (--register)", "page_size": "os.sysconf(SC_PAGE_SIZE)", "physical_cores": "/proc/cpuinfo: unique (physical id, core id) pairs", "ram_total_kib": "/proc/meminfo: MemTotal", "wsl2_config": "no /mnt/c/Users/*/.wslconfig reachable from inside WSL2"}, "wsl2_config": null}
```

  Note: `ram_total_kib` is the RAM **visible inside WSL2** (the
  `.wslconfig` limit when one is set); the Windows host may have more.
  `wsl2_config` is `null` because no `/mnt/c/Users/*/.wslconfig` is
  reachable from inside this WSL2 instance — documented absence, not an
  invention.
- **Periods of use**: from the first deposit (2026-11-17, campaign-1)
  up to the evening of 2026-11-19 (last M1 deposits of the first
  period: perf5, campaign-3, p01 — committed 2026-11-19 15:47 / 15:06
  / 17:01 local); then again from the evening of 2026-11-20 onward
  (switch back from M2).
- **Role**: all PERF-series, campaign-1/2/3, pilot-1, mem19, p01
  measurements; future deposits from 2026-11-20 evening onward.

---

## M2 — more recent machine (shared with other users)

- **Hardware hash**: `unknown` — *the M2 fingerprint was not captured
  while this session ran on M2; fields below are marked as unverified,
  requested from the human, never invented.*
- **Fingerprint**: not captured.
  - CPU model: *unverified — requested from the human*
  - Physical / logical cores: *unverified — requested from the human*
  - RAM visible: *unverified — requested from the human*
  - Kernel: *unverified — requested from the human* (WSL2 expected)
  - WSL2 config: *unverified — requested from the human*
  - Page size: *unverified — requested from the human*
- **Period of use**: from the night of 2026-11-19/20 up to the evening
  of 2026-11-20 (first M2 deposit: align1, committed 2026-11-20 08:04
  local; last M2 deposit: align1/align2 series of 2026-11-20 morning).
- **Role**: ALIGN-1/ALIGN-2 measurements of the 2026-11-20 morning;
  campaign-4 when it runs on M2.

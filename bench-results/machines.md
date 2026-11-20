# Machine register (ALIGN-7b)

Append-only register of the development machines on which
`bench-results/` deposits were measured. One section per machine.

**Rule**: every publication in `bench-results/` carries a `Machine:`
header line (ALIGN-7c). Absolute wall-clock times are only comparable
within one machine; across machines, only ratios are comparable
(ALIGN-7c).

**Switch**: the development machine changed during the evening of
2026-11-19 (old machine M1 → new machine M2). Every deposit committed
up to the evening of 2026-11-19 was measured on **M1**; deposits from
the night of 2026-11-19/20 onward are on **M2**. Deposits committed
during the switch window itself (p11, p12 — see their headers) carry an
explicit uncertainty note.

The fingerprint of each machine is emitted by
`uv run python scripts/machine_fingerprint.py` (ALIGN-7a); the
`hardware_hash` line below is the key the script matches on.

---

## M1 — old development machine

- **Hardware hash**: `unknown` — *requested from the human: the M1
  fingerprint was not captured before the switch; the fields below are
  marked as unverified until answered.*
- **Fingerprint**: not available (machine decommissioned on 2026-11-19
  evening). Fields below are **unverified**, requested from the human:
  - CPU model: *unverified — requested from the human*
  - Physical / logical cores: *unverified — requested from the human*
  - RAM visible: *unverified — requested from the human*
  - Kernel: *unverified — requested from the human* (WSL2 expected)
  - WSL2 config: *unverified — requested from the human*
  - Page size: *unverified — requested from the human*
- **Period of use**: from the first deposit (2026-11-17, campaign-1)
  up to the evening of 2026-11-19 (last M1 deposits: perf5, campaign-3,
  p01 — committed 2026-11-19 15:47 / 15:06 / 17:01 local).
- **Role**: all PERF-series and campaign-1/2/3, pilot-1, mem19, p01
  measurements.

---

## M2 — new development machine (current)

- **Hardware hash**: `febd14ed`
- **Fingerprint** (emitted by `scripts/machine_fingerprint.py` on
  2026-11-20, WSL2):

```json
{"cpu_model": "Intel(R) Core(TM) i5-3470 CPU @ 3.20GHz", "date_utc": "2026-11-20T11:56:45+00:00", "hardware_hash": "febd14ed", "kernel": "6.18.58.1-microsoft-standard-WSL2", "logical_cores": 4, "machine_id": null, "page_size": 4096, "physical_cores": 4, "ram_total_kib": 3979828, "sources": {"cpu_model": "/proc/cpuinfo: model name", "kernel": "uname -r (platform.release())", "logical_cores": "/proc/cpuinfo: count of 'processor' entries", "machine_id": "no register given (--register)", "page_size": "os.sysconf(SC_PAGE_SIZE)", "physical_cores": "/proc/cpuinfo: unique (physical id, core id) pairs", "ram_total_kib": "/proc/meminfo: MemTotal", "wsl2_config": "no /mnt/c/Users/*/.wslconfig reachable from inside WSL2"}, "wsl2_config": null}
```

  Note: `ram_total_kib` is the RAM **visible inside WSL2** (the
  `.wslconfig` limit when one is set); the Windows host may have more.
  `wsl2_config` is `null` because no `/mnt/c/Users/*/.wslconfig` is
  reachable from inside this WSL2 instance — documented absence, not an
  invention.
- **Period of use**: from the night of 2026-11-19/20 onward (first M2
  deposit: align1, 2026-11-20).
- **Role**: ALIGN-series measurements and campaign-4 onward.

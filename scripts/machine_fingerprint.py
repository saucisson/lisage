"""Machine fingerprint for benchmark publications (ALIGN-7a).

Emits one normalized JSON line describing the machine a benchmark ran
on, so that every deposit in ``bench-results/`` can carry a mandatory
``Machine`` header (ALIGN-7c). The short machine identifier is deduced
from the register ``bench-results/machines.md`` (ALIGN-7b) by matching
the hardware hash; an unknown machine gets ``null`` and must be
registered by a human.

The driver never fails: every field is collected independently, and an
unavailable field is emitted as ``null`` with its source (and the error,
when there was one) recorded in ``sources``. Idempotent and fast
(< 1 s): it only reads ``/proc``, ``uname`` and at most one
``.wslconfig``.

Entry point (ADR-0021): ``python -m scripts machine-fingerprint`` or
``make machine-fingerprint``.
"""

import hashlib
import json
import os
import platform
import re
from datetime import UTC, datetime
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    import argparse

REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_REGISTER = REPOSITORY_ROOT / "bench-results" / "machines.md"

CPU_INFO_PATH = Path("/proc/cpuinfo")
MEM_INFO_PATH = Path("/proc/meminfo")
WSL_CONFIG_MOUNT_ROOT = Path("/mnt/c/Users")
HARDWARE_HASH_PREFIX_LENGTH = 8


def _read_text(*, path: Path) -> str | None:
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return None


def _cpu_model(*, sources: dict[str, str]) -> str | None:
    content = _read_text(path=CPU_INFO_PATH)
    if content is None:
        sources["cpu_model"] = f"{CPU_INFO_PATH}: unreadable"
        return None
    match = re.search(r"^model name\s*:\s*(.+)$", content, re.MULTILINE)
    if match is None:
        sources["cpu_model"] = f"{CPU_INFO_PATH}: no 'model name' entry"
        return None
    sources["cpu_model"] = f"{CPU_INFO_PATH}: model name"
    return match.group(1).strip()


def _logical_cores(*, sources: dict[str, str]) -> int | None:
    content = _read_text(path=CPU_INFO_PATH)
    if content is None:
        sources["logical_cores"] = f"{CPU_INFO_PATH}: unreadable"
        return None
    count = len(re.findall(r"^processor\s*:", content, re.MULTILINE))
    if count == 0:
        sources["logical_cores"] = f"{CPU_INFO_PATH}: no 'processor' entries"
        return None
    sources["logical_cores"] = f"{CPU_INFO_PATH}: count of 'processor' entries"
    return count


def _physical_cores(*, sources: dict[str, str]) -> int | None:
    content = _read_text(path=CPU_INFO_PATH)
    if content is None:
        sources["physical_cores"] = f"{CPU_INFO_PATH}: unreadable"
        return None
    # Count unique (physical id, core id) pairs with a per-line parse.
    pairs: set[tuple[str, str]] = set()
    physical_id: str | None = None
    for line in content.splitlines():
        match_id = re.match(r"physical id\s*:\s*(\S+)", line)
        match_core = re.match(r"core id\s*:\s*(\S+)", line)
        if match_id:
            physical_id = match_id.group(1)
        elif match_core and physical_id is not None:
            pairs.add((physical_id, match_core.group(1)))
    if not pairs:
        sources["physical_cores"] = f"{CPU_INFO_PATH}: no (physical id, core id) pairs"
        return None
    sources["physical_cores"] = f"{CPU_INFO_PATH}: unique (physical id, core id) pairs"
    return len(pairs)


def _ram_total_kib(*, sources: dict[str, str]) -> int | None:
    content = _read_text(path=MEM_INFO_PATH)
    if content is None:
        sources["ram_total_kib"] = f"{MEM_INFO_PATH}: unreadable"
        return None
    match = re.search(r"^MemTotal:\s*(\d+)\s*kB", content, re.MULTILINE)
    if match is None:
        sources["ram_total_kib"] = f"{MEM_INFO_PATH}: no MemTotal entry"
        return None
    sources["ram_total_kib"] = f"{MEM_INFO_PATH}: MemTotal"
    return int(match.group(1))


def _kernel(*, sources: dict[str, str]) -> str:
    release = platform.release()
    sources["kernel"] = "uname -r (platform.release())"
    return release


def _wsl2_config(*, sources: dict[str, str]) -> dict[str, str] | None:
    """Best-effort read of the WSL2 host configuration.

    Inside WSL2 the ``.wslconfig`` lives on the Windows host; it is
    reachable through ``/mnt/c/Users/<user>/.wslconfig`` when the Windows
    drive is mounted. Absence is documented, never invented.
    """
    if "microsoft" not in platform.release().lower() and not os.environ.get("WSL_DISTRO_NAME"):
        sources["wsl2_config"] = "not WSL2 (kernel release and WSL_DISTRO_NAME absent)"
        return None
    candidates = (
        sorted(WSL_CONFIG_MOUNT_ROOT.glob("*/.wslconfig")) if WSL_CONFIG_MOUNT_ROOT.is_dir() else []
    )
    if not candidates:
        sources["wsl2_config"] = (
            f"no {WSL_CONFIG_MOUNT_ROOT}/*/.wslconfig reachable from inside WSL2"
        )
        return None
    path = candidates[0]
    content = _read_text(path=path)
    if content is None:
        sources["wsl2_config"] = f"{path}: unreadable"
        return None
    config: dict[str, str] = {"path": str(path)}
    section = ""
    for raw_line in content.splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or line.startswith(";"):
            continue
        section_match = re.match(r"\[(.+)\]", line)
        if section_match:
            section = section_match.group(1).strip().lower()
            continue
        key, separator, value = line.partition("=")
        if not separator:
            continue
        if section == "wsl2":
            config[key.strip().lower()] = value.strip()
    sources["wsl2_config"] = f"{path}: [wsl2] section (best effort)"
    return config


def _page_size(*, sources: dict[str, str]) -> int | None:
    try:
        size = os.sysconf("SC_PAGE_SIZE")
    except (OSError, ValueError) as error:
        sources["page_size"] = f"os.sysconf(SC_PAGE_SIZE): {error}"
        return None
    sources["page_size"] = "os.sysconf(SC_PAGE_SIZE)"
    return int(size)


def _hardware_hash(
    *,
    cpu_model: str | None,
    physical_cores: int | None,
    logical_cores: int | None,
    ram_total_kib: int | None,
) -> str:
    material = "|".join(
        "" if value is None else str(value)
        for value in (cpu_model, physical_cores, logical_cores, ram_total_kib)
    )
    return hashlib.sha256(material.encode()).hexdigest()[:HARDWARE_HASH_PREFIX_LENGTH]


def _machine_id_from_register(*, register_path: Path, hardware_hash: str) -> str | None:
    """Deduce the short machine identifier from the register (ALIGN-7b).

    Each machine section of ``machines.md`` carries its hardware hash in
    a ``Hardware hash:`` line; the section header names the identifier
    (``## M1 — ...``). Unknown hash: ``None``.
    """
    content = _read_text(path=register_path)
    if content is None:
        return None
    current_identifier: str | None = None
    for line in content.splitlines():
        header = re.match(r"^##\s+(\S+)", line)
        if header:
            current_identifier = header.group(1)
            continue
        hash_line = re.search(r"Hardware hash\*?\*?:\s*`([0-9a-f]+)`", line)
        if hash_line and current_identifier is not None and hash_line.group(1) == hardware_hash:
            return current_identifier
    return None


def fingerprint(*, register_path: Path | None) -> dict[str, object]:
    """Collect the fingerprint; never raises."""
    sources: dict[str, str] = {}
    cpu_model = _cpu_model(sources=sources)
    logical_cores = _logical_cores(sources=sources)
    physical_cores = _physical_cores(sources=sources)
    ram_total_kib = _ram_total_kib(sources=sources)
    hardware_hash = _hardware_hash(
        cpu_model=cpu_model,
        physical_cores=physical_cores,
        logical_cores=logical_cores,
        ram_total_kib=ram_total_kib,
    )
    machine_id: str | None = None
    if register_path is not None:
        machine_id = _machine_id_from_register(
            register_path=register_path, hardware_hash=hardware_hash
        )
        sources["machine_id"] = (
            f"{register_path}: hardware hash match"
            if machine_id is not None
            else f"{register_path}: no section with hardware hash {hardware_hash}"
        )
    else:
        sources["machine_id"] = "no register given (--register)"
    return {
        "machine_id": machine_id,
        "hardware_hash": hardware_hash,
        "date_utc": datetime.now(UTC).isoformat(timespec="seconds"),
        "cpu_model": cpu_model,
        "physical_cores": physical_cores,
        "logical_cores": logical_cores,
        "ram_total_kib": ram_total_kib,
        "kernel": _kernel(sources=sources),
        "wsl2_config": _wsl2_config(sources=sources),
        "page_size": _page_size(sources=sources),
        "sources": sources,
    }


def register(subparsers: argparse._SubParsersAction[argparse.ArgumentParser]) -> None:
    """Register the machine-fingerprint subcommand."""
    subparser = subparsers.add_parser(
        "machine-fingerprint",
        help="Emit one normalized JSON line fingerprinting this machine (ALIGN-7a).",
        description="Emit one normalized JSON line fingerprinting this machine (ALIGN-7a).",
    )
    subparser.add_argument(
        "--register",
        type=Path,
        default=DEFAULT_REGISTER,
        help="Path of the machine register (bench-results/machines.md) used to deduce the id.",
    )
    subparser.set_defaults(runner=run)


def run(*, arguments: argparse.Namespace) -> int:
    """Print the fingerprint as one JSON line; never fails."""
    register_path: Path | None = arguments.register if arguments.register.is_file() else None
    print(json.dumps(fingerprint(register_path=register_path), sort_keys=True))
    return 0

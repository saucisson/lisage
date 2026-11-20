"""Tests of the machine fingerprint driver (ALIGN-7a).

The hardware itself is not pinned (it varies per machine); the pure
parts are: the hardware-hash computation, the register parsing that
deduces the machine id, and the never-fail contract (null + source per
unavailable field).
"""

import json
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from pathlib import Path

from scripts.machine_fingerprint import (
    HARDWARE_HASH_PREFIX_LENGTH,
    _hardware_hash,
    _machine_id_from_register,
    _page_size,
    fingerprint,
)

REGISTER_CONTENT = """# Machine register

## M1 — old machine

- **Hardware hash**: `aaaa1111`

## M2 — new machine

- **Hardware hash**: `febd14ed`
"""


def test_hardware_hash_is_deterministic_and_short() -> None:
    first = _hardware_hash(
        cpu_model="cpu", physical_cores=4, logical_cores=8, ram_total_kib=1000
    )
    second = _hardware_hash(
        cpu_model="cpu", physical_cores=4, logical_cores=8, ram_total_kib=1000
    )
    assert first == second
    assert len(first) == HARDWARE_HASH_PREFIX_LENGTH


def test_hardware_hash_changes_with_material() -> None:
    base = _hardware_hash(
        cpu_model="cpu", physical_cores=4, logical_cores=8, ram_total_kib=1000
    )
    other = _hardware_hash(
        cpu_model="cpu", physical_cores=4, logical_cores=8, ram_total_kib=2000
    )
    assert base != other


def test_hardware_hash_tolerates_null_fields() -> None:
    digest = _hardware_hash(
        cpu_model=None, physical_cores=None, logical_cores=None, ram_total_kib=None
    )
    assert len(digest) == HARDWARE_HASH_PREFIX_LENGTH


def test_machine_id_from_register_matches_hash(tmp_path: Path) -> None:
    register = tmp_path / "machines.md"
    register.write_text(REGISTER_CONTENT, encoding="utf-8")
    assert (
        _machine_id_from_register(register_path=register, hardware_hash="febd14ed") == "M2"
    )
    assert (
        _machine_id_from_register(register_path=register, hardware_hash="aaaa1111") == "M1"
    )


def test_machine_id_from_register_unknown_hash_is_null(tmp_path: Path) -> None:
    register = tmp_path / "machines.md"
    register.write_text(REGISTER_CONTENT, encoding="utf-8")
    assert (
        _machine_id_from_register(register_path=register, hardware_hash="00000000") is None
    )


def test_machine_id_from_register_missing_file_is_null(tmp_path: Path) -> None:
    missing = tmp_path / "no-such-register.md"
    assert (
        _machine_id_from_register(register_path=missing, hardware_hash="febd14ed") is None
    )


def test_fingerprint_without_register_has_null_machine_id() -> None:
    result = fingerprint(register_path=None)
    assert result["machine_id"] is None
    sources = result["sources"]
    assert isinstance(sources, dict)
    assert "no register given" in sources["machine_id"]
    # The fingerprint is one JSON-serializable object with the normalized keys.
    assert set(result) == {
        "machine_id",
        "hardware_hash",
        "date_utc",
        "cpu_model",
        "physical_cores",
        "logical_cores",
        "ram_total_kib",
        "kernel",
        "wsl2_config",
        "page_size",
        "sources",
    }
    json.dumps(result)


def test_fingerprint_with_register_deduces_machine_id(tmp_path: Path) -> None:
    register = tmp_path / "machines.md"
    register.write_text(REGISTER_CONTENT, encoding="utf-8")
    result = fingerprint(register_path=register)
    hardware_hash = str(result["hardware_hash"])
    expected = _machine_id_from_register(
        register_path=register, hardware_hash=hardware_hash
    )
    assert result["machine_id"] == expected


def test_page_size_is_positive_or_null_with_source() -> None:
    sources: dict[str, str] = {}
    size = _page_size(sources=sources)
    assert size is None or size > 0
    assert "page_size" in sources


def test_keyword_only_parameters_reject_positional_calls() -> None:
    with pytest.raises(TypeError):
        _hardware_hash("cpu", 4, 8, 1000)  # type: ignore[call-arg]

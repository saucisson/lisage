"""Tests for the commit-subject front scripts/english_only.py (issues #104, #110)."""

import argparse
from typing import TYPE_CHECKING

from scripts import english_only

if TYPE_CHECKING:
    from pathlib import Path


def test_scan_subject_passes_an_english_subject() -> None:
    """An English subject is not an offender."""
    assert english_only.scan_subject(subject="feat(core): saturate in place") is None


def test_scan_subject_flags_an_accented_subject() -> None:
    """An accented subject is an offender, evidenced as an accent."""
    offender = english_only.scan_subject(subject="fix(core): corrige le", )
    assert offender is not None
    assert "accented" in offender.evidence


def test_scan_subject_flags_a_french_stop_word() -> None:
    """An accent-free French subject is caught by its stop-words."""
    offender = english_only.scan_subject(subject="fix: le bug du level")
    assert offender is not None
    assert "French stop-word" in offender.evidence


def test_run_accepts_an_english_subject_file(tmp_path: Path) -> None:
    """The hook shape returns success on an English subject."""
    message_path = tmp_path / "COMMIT_EDITMSG"
    message_path.write_text(
        "feat(core): in-place saturation kernel\n\nBody line.\n", encoding="utf-8"
    )
    arguments = argparse.Namespace(subject_file=str(message_path))
    assert english_only.run(arguments=arguments) == 0


def test_run_refuses_a_french_subject_file(tmp_path: Path) -> None:
    """The hook shape fails on a French subject, whatever the body."""
    message_path = tmp_path / "COMMIT_EDITMSG"
    message_path.write_text("fix: corrige le\nBody.\n", encoding="utf-8")
    arguments = argparse.Namespace(subject_file=str(message_path))
    assert english_only.run(arguments=arguments) == 1


def test_run_refuses_an_unreadable_subject_file(tmp_path: Path) -> None:
    """An unreadable commit message file is a protocol failure."""
    arguments = argparse.Namespace(subject_file=str(tmp_path / "absent"))
    assert english_only.run(arguments=arguments) == 2

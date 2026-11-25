"""Tests for the reviewer session driver scripts/session.py (ADR-0010).

Ported from project-template PR 28, adapted to the ADR-0010 naming of
this repository. The driver never trusts the session: counts are
recomputed locally, an unparsable answer fails closed, and an unknown
severity becomes a BLOCKING finding - never a benefit of the doubt.
"""

import json

import pytest

from scripts import session
from scripts.session import FindingSeverity, ReviewFinding, ReviewProtocolError


def _valid_answer(findings: list[dict[str, object]]) -> str:
    """A well-formed session answer with the given findings."""
    return json.dumps({"findings": findings, "recommendation": "merge"})


def _finding_entry(severity: str = "BLOCKING") -> dict[str, object]:
    """One finding entry as the session writes it."""
    return {
        "severity": severity,
        "file": "scripts/day.py",
        "line": 42,
        "rule": "the violated rule",
        "evidence": "the concrete counterexample",
    }


def test_narrow_json_answer_keeps_a_bare_object() -> None:
    """A bare JSON object passes through unchanged."""
    answer = '{"findings": []}'
    assert session.narrow_json_answer(answer_text=answer) == answer


def test_narrow_json_answer_strips_code_fences() -> None:
    """A fenced json answer is narrowed to its payload."""
    answer = '```json\n{"findings": []}\n```'
    assert session.narrow_json_answer(answer_text=answer) == '{"findings": []}'


def test_narrow_json_answer_narrows_surrounding_prose() -> None:
    """Prose around the object is discarded, the object kept."""
    answer = 'Here you go: {"findings": []} - that is all.'
    assert session.narrow_json_answer(answer_text=answer) == '{"findings": []}'


def test_narrow_json_answer_fails_closed_without_an_object() -> None:
    """No brace at all is a protocol error."""
    with pytest.raises(ReviewProtocolError):
        session.narrow_json_answer(answer_text="nothing here")


def test_parse_report_accepts_zero_findings() -> None:
    """Zero findings is a valid answer: an empty list."""
    assert session.parse_report(answer_text=_valid_answer([])) == []


def test_parse_report_parses_one_valid_finding() -> None:
    """A valid finding round-trips into a ReviewFinding."""
    findings = session.parse_report(answer_text=_valid_answer([_finding_entry()]))
    assert len(findings) == 1
    assert findings[0] == ReviewFinding(
        severity=FindingSeverity.BLOCKING,
        file="scripts/day.py",
        line=42,
        rule="the violated rule",
        evidence="the concrete counterexample",
    )


def test_parse_report_fails_closed_on_invalid_json() -> None:
    """An answer that is not JSON is a protocol error."""
    with pytest.raises(ReviewProtocolError):
        session.parse_report(answer_text="not json")


def test_parse_report_fails_closed_without_a_findings_list() -> None:
    """No findings key in the object is a protocol error."""
    with pytest.raises(ReviewProtocolError):
        session.parse_report(answer_text='{"recommendation": "merge"}')


def test_parse_finding_fails_closed_without_an_integer_line() -> None:
    """A non-integer line is a protocol error, never a guess."""
    entry = _finding_entry()
    entry["line"] = "42"
    with pytest.raises(ReviewProtocolError):
        session.parse_finding(finding_container=entry)


def test_parse_finding_blocks_an_unknown_severity() -> None:
    """An unreadable severity becomes BLOCKING: fail closed."""
    entry = _finding_entry(severity="CATASTROPHIC")
    finding = session.parse_finding(finding_container=entry)
    assert finding.severity is FindingSeverity.BLOCKING
    assert finding.rule == "CATASTROPHIC"


def test_parse_finding_accepts_every_documented_severity() -> None:
    """The three severities of the ladder all parse."""
    for severity, expected in (
        ("BLOCKING", FindingSeverity.BLOCKING),
        ("TO-FIX", FindingSeverity.TO_FIX),
        ("SUGGESTION", FindingSeverity.SUGGESTION),
    ):
        finding = session.parse_finding(finding_container=_finding_entry(severity=severity))
        assert finding.severity is expected


def test_count_by_severity_recounts_locally() -> None:
    """The counts come from a local recount, never from the session."""
    findings = [
        session.parse_finding(finding_container=_finding_entry()) for _ in range(3)
    ]
    suggestions = [
        session.parse_finding(finding_container=_finding_entry(severity="SUGGESTION"))
        for _ in range(2)
    ]
    counts = session.count_by_severity(findings=findings + suggestions)
    assert counts[FindingSeverity.BLOCKING] == 3
    assert counts[FindingSeverity.SUGGESTION] == 2
    assert counts[FindingSeverity.TO_FIX] == 0


def test_count_by_severity_starts_from_zero_findings() -> None:
    """Zero findings recount to zero, on every severity."""
    counts = session.count_by_severity(findings=[])
    assert set(counts.values()) == {0}

"""Tests for the CI wait driver scripts/ci.py (ADR-0018, ADR-0108).

Ported from project-template PR 28, adapted to the gh-first signatures
of this repository: github_request is monkeypatched and routes by
endpoint (pull request fetch versus check-runs list), exactly the two
operations poll_once performs. Every verdict of the wait is pinned:
green, failed, conflict, no checks, pending, transient refusal.
"""

from typing import TYPE_CHECKING
from unittest import mock

import pytest

from scripts import ci
from scripts._github import HTTP_SUCCESS_STATUS_CODE
from scripts._shared import EXIT_CODE_CRITERIA_UNMET, EXIT_CODE_PROTOCOL_FAILURE, EXIT_CODE_SUCCESS

if TYPE_CHECKING:
    from collections.abc import Sequence


def _pull_request(
    *,
    head_sha: str = "abc123",
    mergeable_state: str = "clean",
    mergeable: bool = True,
) -> dict[str, object]:
    """A mergeable pull request payload with a known head."""
    return {
        "state": "open",
        "mergeable": mergeable,
        "mergeable_state": mergeable_state,
        "head": {"sha": head_sha, "ref": "feature"},
        "base": {"ref": "daily"},
    }


def _check_runs_payload(check_runs: Sequence[dict[str, object]]) -> dict[str, object]:
    """A check-runs API payload."""
    return {"check_runs": list(check_runs)}


def _patch_github(
    *,
    pull_request: dict[str, object],
    check_runs: Sequence[dict[str, object]],
):
    """Monkeypatch github_request, routing pulls versus check-runs."""
    def fake_request(*, endpoint: str, method: str = "GET", json_payload: object = None):
        if endpoint.endswith("check-runs"):
            return HTTP_SUCCESS_STATUS_CODE, _check_runs_payload(check_runs)
        return HTTP_SUCCESS_STATUS_CODE, pull_request

    return mock.patch.object(ci, "github_request", side_effect=fake_request)


def _green_check_runs() -> list[dict[str, object]]:
    return [{"name": "quality gate", "status": "completed", "conclusion": "success"}]


def test_poll_once_reports_green_ci() -> None:
    """Every check run concluded success: the wait ends green."""
    with _patch_github(pull_request=_pull_request(), check_runs=_green_check_runs()):
        status, head_sha, detail = ci.poll_once(
            repository_slug="saucisson/lisage",
            pull_request_number=7,
        )
    assert status == ci.POLL_STATUS_GREEN
    assert head_sha == "abc123"
    assert detail == "1"


def test_poll_once_reports_failed_ci() -> None:
    """A completed check run without success fails the wait."""
    check_runs = [
        {"name": "quality gate", "status": "completed", "conclusion": "failure"},
    ]
    with _patch_github(pull_request=_pull_request(), check_runs=check_runs):
        status, _, detail = ci.poll_once(
            repository_slug="saucisson/lisage",
            pull_request_number=7,
        )
    assert status == ci.POLL_STATUS_FAILED
    assert "quality gate" in detail


def test_poll_once_names_a_merge_conflict_first() -> None:
    """A dirty state is a conflict, named before any CI verdict (ADR-0108)."""
    check_runs = [
        {"name": "quality gate", "status": "in_progress", "conclusion": None},
    ]
    with _patch_github(
        pull_request=_pull_request(mergeable_state="dirty", mergeable=False),
        check_runs=check_runs,
    ):
        status, _, _ = ci.poll_once(
            repository_slug="saucisson/lisage",
            pull_request_number=7,
        )
    assert status == ci.POLL_STATUS_CONFLICT


def test_poll_once_reports_pending_ci() -> None:
    """A still-running check run keeps the wait going."""
    check_runs = [
        {"name": "quality gate", "status": "in_progress", "conclusion": None},
    ]
    with _patch_github(pull_request=_pull_request(), check_runs=check_runs):
        status, _, detail = ci.poll_once(
            repository_slug="saucisson/lisage",
            pull_request_number=7,
        )
    assert status == ci.POLL_STATUS_PENDING
    assert "1 running" in detail


def test_poll_once_reports_no_check_runs() -> None:
    """No check run on the head yet: no-checks, never a green."""
    with _patch_github(pull_request=_pull_request(), check_runs=[]):
        status, head_sha, _ = ci.poll_once(
            repository_slug="saucisson/lisage",
            pull_request_number=7,
        )
    assert status == ci.POLL_STATUS_NO_CHECKS
    assert head_sha == "abc123"


def test_poll_once_reports_a_transient_refusal() -> None:
    """A transient API refusal is retried, never mistaken for green."""
    with mock.patch.object(
        ci,
        "github_request",
        return_value=(502, None),
    ):
        status, head_sha, _ = ci.poll_once(
            repository_slug="saucisson/lisage",
            pull_request_number=7,
        )
    assert status == ci.POLL_STATUS_FETCH_FAILED
    assert head_sha == ""


def test_try_fetch_pull_request_fails_closed_on_a_missing_pr() -> None:
    """A 404 is a protocol failure: nothing was watchable."""
    with mock.patch.object(
        ci,
        "github_request",
        return_value=(ci.NOT_FOUND_STATUS_CODE, {"message": "Not Found"}),
    ):
        with pytest.raises(SystemExit) as exit_information:
            ci.try_fetch_pull_request(
                repository_slug="saucisson/lisage",
                pull_request_number=99,
            )
    assert exit_information.value.code == EXIT_CODE_PROTOCOL_FAILURE


def test_interpret_status_ends_the_wait_on_green() -> None:
    """Green exits 0: the review can start."""
    assert (
        ci.interpret_status(status=ci.POLL_STATUS_GREEN, detail="3")
        == EXIT_CODE_SUCCESS
    )


def test_interpret_status_ends_the_wait_on_failure() -> None:
    """A failed CI exits 1: fix, push, wait again."""
    assert (
        ci.interpret_status(status=ci.POLL_STATUS_FAILED, detail="quality gate")
        == EXIT_CODE_CRITERIA_UNMET
    )


def test_interpret_status_ends_the_wait_on_conflict() -> None:
    """A merge conflict exits 1 with the conflict named."""
    assert (
        ci.interpret_status(status=ci.POLL_STATUS_CONFLICT, detail="")
        == EXIT_CODE_CRITERIA_UNMET
    )


def test_interpret_status_keeps_waiting_otherwise() -> None:
    """No checks, pending, and transient errors all return None: wait on."""
    for status in (ci.POLL_STATUS_NO_CHECKS, ci.POLL_STATUS_PENDING, ci.POLL_STATUS_FETCH_FAILED):
        assert ci.interpret_status(status=status, detail="") is None


def test_run_watch_returns_green_when_ci_concludes_green(monkeypatch) -> None:
    """The loop: one pending poll, then green - exit 0."""
    polls = [
        (ci.POLL_STATUS_PENDING, "abc123", "1 running, 0 green"),
        (ci.POLL_STATUS_GREEN, "abc123", "1"),
    ]
    monkeypatch.setattr(ci, "poll_once", lambda **_kwargs: polls.pop(0))
    assert (
        ci.run_watch(
            repository_slug="saucisson/lisage",
            pull_request_number=7,
            interval_seconds=1,
            timeout_seconds=60,
        )
        == EXIT_CODE_SUCCESS
    )


def test_run_watch_returns_ci_failed_on_a_conflict(monkeypatch) -> None:
    """A conflict observed mid-wait exits 1 immediately."""
    monkeypatch.setattr(
        ci,
        "poll_once",
        lambda **_kwargs: (ci.POLL_STATUS_CONFLICT, "", ""),
    )
    assert (
        ci.run_watch(
            repository_slug="saucisson/lisage",
            pull_request_number=7,
            interval_seconds=1,
            timeout_seconds=60,
        )
        == EXIT_CODE_CRITERIA_UNMET
    )

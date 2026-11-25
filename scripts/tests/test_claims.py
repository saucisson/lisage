"""Tests for the claim driver scripts/claims.py (ADR-0017).

Ported from project-template PR 28: the claim is both the concurrency
guard and the honesty record, so every refusal path is pinned here -
never claim twice, never work the wrong kind, never work unclaimed.
"""

from typing import TYPE_CHECKING
from unittest import mock

import pytest

from scripts import claims
from scripts._shared import EXIT_CODE_CRITERIA_UNMET, EXIT_CODE_PROTOCOL_FAILURE

if TYPE_CHECKING:
    from collections.abc import Sequence


def _issue(labels: Sequence[str] | None = None) -> dict[str, object]:
    """A plain, unclaimed issue as the GitHub API returns it."""
    return {
        "number": 12,
        "pull_request": None,
        "labels": [{"name": name} for name in (labels or [])],
    }


def _pull_request(labels: Sequence[str] | None = None) -> dict[str, object]:
    """A pull request payload: the issues endpoint serves both kinds."""
    issue = _issue(labels)
    issue["pull_request"] = {"url": "https://api.github.com/repos/x/y/pulls/12"}
    return issue


def test_claim_is_allowed_accepts_an_unclaimed_issue() -> None:
    """An open, unclaimed issue is claimable."""
    claims.claim_is_allowed(issue=_issue(), item_number=12, expected_kind="issue")


def test_claim_is_allowed_accepts_an_unclaimed_pull_request() -> None:
    """An open, unclaimed pull request is claimable (recovery path)."""
    claims.claim_is_allowed(issue=_pull_request(), item_number=12, expected_kind="pull_request")


def test_claim_is_allowed_refuses_a_kind_mismatch_issue_as_pr() -> None:
    """An issue number passed to --pr fails closed."""
    with pytest.raises(SystemExit) as exit_information:
        claims.claim_is_allowed(issue=_issue(), item_number=12, expected_kind="pull_request")
    assert exit_information.value.code == EXIT_CODE_CRITERIA_UNMET


def test_claim_is_allowed_refuses_a_kind_mismatch_pr_as_issue() -> None:
    """A pull request number passed to --issue fails closed."""
    with pytest.raises(SystemExit) as exit_information:
        claims.claim_is_allowed(issue=_pull_request(), item_number=12, expected_kind="issue")
    assert exit_information.value.code == EXIT_CODE_CRITERIA_UNMET


def test_claim_is_allowed_refuses_an_already_claimed_item() -> None:
    """A claimed item is the human's decision now, never a re-claim."""
    issue = _issue(labels=[claims.CLAIM_LABEL_NAME])
    with pytest.raises(SystemExit) as exit_information:
        claims.claim_is_allowed(issue=issue, item_number=12, expected_kind="issue")
    assert exit_information.value.code == EXIT_CODE_CRITERIA_UNMET


def test_claim_is_allowed_ignores_other_labels() -> None:
    """Unrelated labels never block a claim."""
    claims.claim_is_allowed(
        issue=_issue(labels=["needs-human"]),
        item_number=12,
        expected_kind="issue",
    )


def test_ensure_claim_label_returns_when_the_label_exists() -> None:
    """An existing label means nothing to create."""
    with mock.patch.object(
        claims,
        "github_request",
        return_value=(200, {"name": claims.CLAIM_LABEL_NAME}),
    ):
        claims.ensure_claim_label(repository_slug="saucisson/lisage")


def test_ensure_claim_label_creates_a_missing_label() -> None:
    """A 404 (not 200) followed by a creation is the happy path."""
    responses = [(404, {"message": "Not Found"}), (201, {})]
    with mock.patch.object(claims, "github_request", side_effect=responses):
        claims.ensure_claim_label(repository_slug="saucisson/lisage")


def test_ensure_claim_label_fails_closed_without_permission() -> None:
    """A 403 on creation is a human decision, so the protocol fails."""
    responses = [(404, {"message": "Not Found"}), (403, {"message": "Forbidden"})]
    with mock.patch.object(claims, "github_request", side_effect=responses):
        with pytest.raises(SystemExit) as exit_information:
            claims.ensure_claim_label(repository_slug="saucisson/lisage")
    assert exit_information.value.code == EXIT_CODE_PROTOCOL_FAILURE


def test_add_claim_label_accepts_a_successful_addition() -> None:
    """A 200 on the addition is a claimed item."""
    with mock.patch.object(claims, "github_request", return_value=(200, {})):
        claims.add_claim_label(repository_slug="saucisson/lisage", issue_number=12)


def test_add_claim_label_fails_closed_on_refusal() -> None:
    """No claim, no work: a refused addition fails the protocol."""
    with mock.patch.object(claims, "github_request", return_value=(422, {"message": "Validation"})):
        with pytest.raises(SystemExit) as exit_information:
            claims.add_claim_label(repository_slug="saucisson/lisage", issue_number=12)
    assert exit_information.value.code == EXIT_CODE_PROTOCOL_FAILURE

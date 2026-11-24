"""Claim driver: mark one issue or pull request taken (ADR-0017).

In the auto-discovered night loop (``make night`` without ISSUES) each
session picks its own item, so its FIRST act is to claim it:
``make claim ISSUE=N`` or ``make claim-pr PR=N`` adds the
``night-claimed`` label through the GitHub API (gh client first, REST
fallback, ADR-0022; the issues endpoint serves pull requests too).
The claim is both the concurrency guard and the honesty record:

- two sessions never work the same item (the loop filters claimed
  items out of every worklist, and this driver refuses to claim
  twice);
- a failed attempt stays claimed: the morning audit sees what was
  tried, and the human - never an agent - decides the retry.

Pull-request claims cover the recovery path (ADR-0017): a session may
be handed - or pick - a pull request left open by an earlier failed
session (red CI, unaddressed findings). The same rule applies: no
claim, no work.
"""

import sys
from typing import TYPE_CHECKING

from scripts._github import fetch_issue, github_request
from scripts._shared import (
    CLAIM_LABEL_NAME,
    EXIT_CODE_CRITERIA_UNMET,
    EXIT_CODE_PROTOCOL_FAILURE,
    EXIT_CODE_SUCCESS,
    resolve_repository_slug,
)

if TYPE_CHECKING:
    import argparse

CLAIM_LABEL_COLOR = "d4c5f9"
CLAIM_LABEL_DESCRIPTION = "Claimed by an unattended night session (ADR-0017)"
HTTP_SUCCESS_STATUS_CODE = 200
EXIT_CODE_CLAIMED = EXIT_CODE_SUCCESS  # a subcommand-local alias, same protocol code


def claim_is_allowed(
    *,
    issue: dict[str, object],
    item_number: int,
    expected_kind: str,
) -> None:
    """Refuse a kind mismatch or an already-claimed item.

    ``expected_kind`` is ``"issue"`` or ``"pull_request"``: the payload
    must be exactly that kind (the issues endpoint serves both), so an
    issue number passed to ``--pr`` - or the reverse - fails closed.
    """
    is_pull_request = "pull_request" in issue
    if expected_kind == "pull_request" and not is_pull_request:
        print(f"Error: #{item_number} is an issue, not a pull request.", file=sys.stderr)
        raise SystemExit(EXIT_CODE_CRITERIA_UNMET)
    if expected_kind == "issue" and is_pull_request:
        print(f"Error: #{item_number} is a pull request, not an issue.", file=sys.stderr)
        raise SystemExit(EXIT_CODE_CRITERIA_UNMET)
    labels = issue.get("labels", [])
    if isinstance(labels, list):
        label_names = {entry.get("name", "") for entry in labels if isinstance(entry, dict)}
        if CLAIM_LABEL_NAME in label_names:
            print(
                f"Error: #{item_number} is already claimed"
                f" ({CLAIM_LABEL_NAME}) - do not work it; the human decides"
                " the retry.",
                file=sys.stderr,
            )
            raise SystemExit(EXIT_CODE_CRITERIA_UNMET)


def ensure_claim_label(*, repository_slug: str) -> None:
    """Create the claim label once; a 403 means the human creates it."""
    status_code, _ = github_request(endpoint=f"repos/{repository_slug}/labels/{CLAIM_LABEL_NAME}")
    if status_code == HTTP_SUCCESS_STATUS_CODE:
        return
    creation_status, _ = github_request(
        endpoint=f"repos/{repository_slug}/labels",
        method="POST",
        json_payload={
            "name": CLAIM_LABEL_NAME,
            "color": CLAIM_LABEL_COLOR,
            "description": CLAIM_LABEL_DESCRIPTION,
        },
    )
    if creation_status == HTTP_SUCCESS_STATUS_CODE:
        return
    print(
        f"Error: could not create the {CLAIM_LABEL_NAME} label"
        f" ({creation_status}) - create it by hand or grant"
        " issues write on the token, then relaunch the night.",
        file=sys.stderr,
    )
    raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE)


def add_claim_label(
    *,
    repository_slug: str,
    issue_number: int,
) -> None:
    """Add the claim label; a failure means no claim, no work."""
    status_code, _ = github_request(
        endpoint=f"repos/{repository_slug}/issues/{issue_number}/labels",
        method="POST",
        json_payload={"labels": [CLAIM_LABEL_NAME]},
    )
    if status_code != HTTP_SUCCESS_STATUS_CODE:
        print(
            f"Error: could not claim #{issue_number}"
            f" ({status_code}) - do not work an unclaimed item.",
            file=sys.stderr,
        )
        raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE)


def register(subparsers: argparse._SubParsersAction[argparse.ArgumentParser]) -> None:
    """Register the claim subcommand: exactly one item, issue or pull request."""
    subparser = subparsers.add_parser(
        "claim",
        help="Claim one issue or pull request for an unattended session (ADR-0017).",
        description="Claim one issue or pull request for an unattended session (ADR-0017).",
    )
    item_group = subparser.add_mutually_exclusive_group(required=True)
    item_group.add_argument(
        "--issue",
        type=int,
        help="Issue number to claim (the session's first act).",
    )
    item_group.add_argument(
        "--pr",
        type=int,
        help="Pull request number to claim (recovery path, ADR-0017).",
    )
    subparser.set_defaults(runner=run)


def run(*, arguments: argparse.Namespace) -> int:
    """Entry point: item number in, claimed item out."""
    repository_slug = resolve_repository_slug()
    if arguments.issue is not None:
        item_number, expected_kind, item_word = arguments.issue, "issue", "Issue"
    else:
        item_number, expected_kind, item_word = arguments.pr, "pull_request", "Pull request"
    issue = fetch_issue(
        repository_slug=repository_slug,
        item_number=item_number,
    )
    claim_is_allowed(
        issue=issue,
        item_number=item_number,
        expected_kind=expected_kind,
    )
    ensure_claim_label(repository_slug=repository_slug)
    add_claim_label(
        repository_slug=repository_slug,
        issue_number=item_number,
    )
    print(
        f"{item_word} #{item_number} claimed ({CLAIM_LABEL_NAME}):"
        " it is yours, and no other session will pick it."
    )
    return EXIT_CODE_CLAIMED

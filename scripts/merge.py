"""Merge driver: merge one reviewed pull request into daily (ADR-0009).

The developer agent merges each completed pull request into ``daily``
- through this driver and nothing else. The driver IS the codified
merge criteria: it refuses to merge unless the pull request targets
`daily` only (an agent never merges `main`; A1), the review report
exists, was written for THIS pull request and its CURRENT head (review
context), contains zero BLOCKING findings, the pull request is open
and mergeable (a merge conflict is named for what it is), and every
check run on its head has concluded "success". Then it merges with
the linear method the repository allows - squash preferred, a rebase
merge as the fallback when squash is disabled; a merge commit is
never issued (linear history on ``daily``, ADR-0009) - and deletes
the branch.

Every GitHub operation goes through the gh client first, with the
REST API as the fallback when gh fails (ADR-0108); the merge method is
pinned ``sha`` to the reviewed head, as before. The base check is
ported from project-template PR 27: the driver verified every criterion
except the one that decides WHERE the merge lands - fail-open against
A1, now fail-closed in the head of evaluate_merge_criteria.

The reviewer session never merges: it writes the report; this driver
reads it. ``main`` is never merged here - that stays a human action.
"""

import json
import sys
from pathlib import Path
from typing import TYPE_CHECKING

from scripts._github import (
    HTTP_CREATED_STATUS_CODE,
    HTTP_NO_CONTENT_STATUS_CODE,
    HTTP_SUCCESS_STATUS_CODE,
    check_runs_allow_merge,
    extract_head_ref,
    extract_head_sha,
    fetch_check_runs,
    fetch_pull_request,
    github_request,
    pull_request_has_merge_conflict,
)
from scripts._shared import (
    EXIT_CODE_CRITERIA_UNMET,
    EXIT_CODE_PROTOCOL_FAILURE,
    EXIT_CODE_SUCCESS,
    resolve_repository_slug,
)

if TYPE_CHECKING:
    import argparse
    from collections.abc import Sequence

DEFAULT_REPORT_FILE_NAME = ".review/review-report.json"
DEFAULT_CONTEXT_FILE_NAME = ".review/review-context.json"
BASE_BRANCH_NAME = "daily"  # agent merges land on daily, never main
MERGE_METHOD_PREFERENCE = ("squash", "rebase")
METHOD_NOT_ALLOWED_STATUS_CODE = 405
EXIT_CODE_MERGED = EXIT_CODE_SUCCESS  # a subcommand-local alias, same protocol code


def load_review_report(*, report_file: str) -> dict[str, object]:
    """Load the review report; missing means criteria unmet, not failure."""
    report_path = Path(report_file)
    if not report_path.is_file():
        print(
            f"Error: no review report at {report_file} - run"
            " `make review PR=...` first; merging without a review is"
            " never an option.",
            file=sys.stderr,
        )
        raise SystemExit(EXIT_CODE_CRITERIA_UNMET)
    try:
        report: object = json.loads(report_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as report_error:
        print(
            f"Error: the review report is unreadable: {report_error}",
            file=sys.stderr,
        )
        raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE) from report_error
    if not isinstance(report, dict):
        raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE)
    return report


def load_review_context(*, context_file: str) -> dict[str, object]:
    """Load the review context; missing means criteria unmet, not failure."""
    context_path = Path(context_file)
    if not context_path.is_file():
        print(
            f"Error: no review context at {context_file} - run"
            " `make review PR=...` first; merging without a review is"
            " never an option.",
            file=sys.stderr,
        )
        raise SystemExit(EXIT_CODE_CRITERIA_UNMET)
    try:
        context: object = json.loads(context_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as context_error:
        print(
            f"Error: the review context is unreadable: {context_error}",
            file=sys.stderr,
        )
        raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE) from context_error
    if not isinstance(context, dict):
        raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE)
    return context


def check_review_context(
    *,
    context: dict[str, object],
    pull_request_number: int,
    head_sha: str,
) -> None:
    """Verify the report belongs to this pull request and this head.

    Raises SystemExit with EXIT_CODE_CRITERIA_UNMET on any mismatch: a
    report written for another pull request, or before new commits were
    pushed, is stale and never a merge ticket.
    """
    context_pr_number = context.get("pr_number")
    if context_pr_number != pull_request_number:
        print(
            f"Merge refused: the review report was written for PR"
            f" {context_pr_number}, not PR {pull_request_number} - rerun"
            " `make review` on this pull request.",
            file=sys.stderr,
        )
        raise SystemExit(EXIT_CODE_CRITERIA_UNMET)
    context_head_sha = context.get("head_sha")
    if not isinstance(context_head_sha, str) or not context_head_sha:
        print(
            "Merge refused: the review context has no head SHA - rerun `make review`.",
            file=sys.stderr,
        )
        raise SystemExit(EXIT_CODE_CRITERIA_UNMET)
    if context_head_sha != head_sha:
        print(
            "Merge refused: commits were pushed after the review - rerun"
            " `make review` on the new head.",
            file=sys.stderr,
        )
        raise SystemExit(EXIT_CODE_CRITERIA_UNMET)


def resolve_merge_method(*, repository_slug: str) -> str:
    """Return the linear merge method the repository allows (ADR-0009).

    Squash is preferred (one commit per pull request, the whole review
    trace in one place); a rebase merge is the accepted fallback when
    the repository forbids squash - both keep the history of ``daily``
    linear, so no merge commit is ever issued. A repository allowing
    neither is a human decision away, not an agent one.
    """
    status_code, payload = github_request(endpoint=f"repos/{repository_slug}")
    if status_code != HTTP_SUCCESS_STATUS_CODE or not isinstance(payload, dict):
        print(
            f"Error: could not fetch the settings of {repository_slug}"
            f" through gh (status {status_code}).",
            file=sys.stderr,
        )
        raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE)
    allowed_methods = {
        "squash": payload.get("allow_squash_merge"),
        "rebase": payload.get("allow_rebase_merge"),
    }
    for method in MERGE_METHOD_PREFERENCE:
        if allowed_methods.get(method) is True:
            return method
    print(
        "Merge refused: the repository allows neither squash nor rebase"
        " merges - enabling one of the two is a human decision"
        " (repository settings), not an agent one.",
        file=sys.stderr,
    )
    raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE)


def extract_base_ref(*, pull_request: dict[str, object]) -> str:
    """Return the pull request base branch name, empty when unknown."""
    base_container = pull_request.get("base")
    return base_container.get("ref", "") if isinstance(base_container, dict) else ""


def evaluate_merge_criteria(  # noqa: PLR0911 - one return per codified criterion (ADR-0009)
    *,
    report: dict[str, object],
    pull_request: dict[str, object],
    check_runs: Sequence[dict[str, object]],
) -> tuple[bool, str]:
    """Apply the codified merge criteria (ADR-0009); return (ok, reason)."""
    base_ref = extract_base_ref(pull_request=pull_request)
    if base_ref != BASE_BRANCH_NAME:
        return False, (
            f"the pull request targets \`{base_ref or 'an unknown branch'}\`"
            f", not \`{BASE_BRANCH_NAME}\` - agent tooling merges into"
            " \`daily\` only (A1: no agent ever merges \`main\`)"
        )
    blocking_count = report.get("blocking_count")
    if blocking_count is None:
        return False, "the review report has no blocking_count - rerun the review"
    if blocking_count != 0:
        return False, (
            f"the review report still has {blocking_count} BLOCKING"
            " findings - iterate until the review is clean"
        )
    if pull_request.get("state") != "open":
        return False, "the pull request is not open"
    if pull_request.get("draft") is True:
        return False, "the pull request is a draft"
    if pull_request_has_merge_conflict(pull_request=pull_request):
        return False, (
            "the pull request has a merge conflict - rebase it on daily and rerun the review"
        )
    if pull_request.get("mergeable") is not True:
        return False, (
            "the pull request is not cleanly mergeable - rebase it on daily and rerun the review"
        )
    refusal_reason = check_runs_allow_merge(check_runs=check_runs)
    if refusal_reason is not None:
        return False, refusal_reason
    return True, "criteria satisfied: review clean, CI green, PR mergeable"


def merge_pull_request(
    *,
    repository_slug: str,
    pull_request_number: int,
    head_sha: str,
    merge_method: str,
) -> bool:
    """Merge the pull request with the given linear method; fail closed."""
    status_code, payload = github_request(
        endpoint=f"repos/{repository_slug}/pulls/{pull_request_number}/merge",
        method="PUT",
        json_payload={"sha": head_sha, "merge_method": merge_method},
    )
    if status_code in (HTTP_SUCCESS_STATUS_CODE, HTTP_CREATED_STATUS_CODE):
        return True
    if status_code == METHOD_NOT_ALLOWED_STATUS_CODE:
        message = (
            payload.get("message", "not mergeable")
            if isinstance(payload, dict)
            else "not mergeable"
        )
        print(f"Merge refused by GitHub: {message}.", file=sys.stderr)
        raise SystemExit(EXIT_CODE_CRITERIA_UNMET)
    print(
        f"Error: the merge API returned {status_code}: {payload}",
        file=sys.stderr,
    )
    raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE)


def delete_branch(
    *,
    repository_slug: str,
    head_ref: str,
) -> None:
    """Delete the merged branch; best effort, never a merge failure."""
    if not head_ref:
        return
    status_code, _ = github_request(
        endpoint=f"repos/{repository_slug}/git/refs/heads/{head_ref}",
        method="DELETE",
    )
    if status_code not in (HTTP_SUCCESS_STATUS_CODE, HTTP_NO_CONTENT_STATUS_CODE):
        print(f"Warning: could not delete branch {head_ref} (status {status_code}).")


def register(subparsers: argparse._SubParsersAction[argparse.ArgumentParser]) -> None:
    """Register the merge-pr subcommand."""
    subparser = subparsers.add_parser(
        "merge-pr",
        help="Merge one reviewed pull request into daily (ADR-0009).",
        description="Merge one reviewed pull request into daily (ADR-0009).",
    )
    subparser.add_argument(
        "--pr-number",
        type=int,
        required=True,
        help="Number of the pull request to merge.",
    )
    subparser.add_argument(
        "--report-file",
        default=DEFAULT_REPORT_FILE_NAME,
        help="Path to the review report produced by make review.",
    )
    subparser.add_argument(
        "--context-file",
        default=DEFAULT_CONTEXT_FILE_NAME,
        help="Path to the review context produced by make review.",
    )
    subparser.set_defaults(runner=run)


def run(*, arguments: argparse.Namespace) -> int:
    """Entry point: criteria checked, pull request merged, branch deleted."""
    repository_slug = resolve_repository_slug()
    report = load_review_report(report_file=arguments.report_file)
    pull_request = fetch_pull_request(
        repository_slug=repository_slug,
        pull_request_number=arguments.pr_number,
    )
    head_sha = extract_head_sha(pull_request=pull_request)
    head_ref = extract_head_ref(pull_request=pull_request)
    context = load_review_context(context_file=arguments.context_file)
    check_review_context(
        context=context,
        pull_request_number=arguments.pr_number,
        head_sha=head_sha,
    )
    check_runs = fetch_check_runs(
        repository_slug=repository_slug,
        head_sha=head_sha,
    )
    criteria_met, reason = evaluate_merge_criteria(
        report=report,
        pull_request=pull_request,
        check_runs=check_runs,
    )
    if not criteria_met:
        print(f"Merge refused: {reason}.")
        return EXIT_CODE_CRITERIA_UNMET
    merge_method = resolve_merge_method(repository_slug=repository_slug)
    print(f"Criteria met ({reason}); merging PR #{arguments.pr_number}.")
    merge_pull_request(
        repository_slug=repository_slug,
        pull_request_number=arguments.pr_number,
        head_sha=head_sha,
        merge_method=merge_method,
    )
    delete_branch(
        repository_slug=repository_slug,
        head_ref=head_ref,
    )
    print(f"PR #{arguments.pr_number} merged into daily ({merge_method} merge); branch deleted.")
    return EXIT_CODE_MERGED

"""GitHub access for the drivers: the gh client first, REST fallback (ADR-0108).

Drivers never hand-code HTTP against the GitHub API anymore: the gh
client owns authentication, and one ``gh api`` subprocess replaces the
per-driver HTTP wiring (ADR-0108, ported from project-template
ADR-0008). Every helper below is gh-first: ``gh api <endpoint>``
returns the JSON payload. When gh is missing, unsigned, or fails for
ANY reason, the call falls back to exactly one raw REST request built
here with ``GITHUB_TOKEN``; if the fallback fails too, the driver
fails closed.

The helpers stay endpoint-shaped (``repos/<slug>/pulls/<N>``) on
purpose: the REST fallback converses with the same API gh speaks, and
the drivers read like the operations they name. The modules that never
touch GitHub (``day-report``, ``sync-daily``, ``review-bundle``) do not
import this module, so they stay stdlib-only (ADR-0021).
"""

import json
import subprocess
import sys
from typing import TYPE_CHECKING

import httpx

from scripts._shared import (
    EXIT_CODE_PROTOCOL_FAILURE,
    EXIT_CODE_SUCCESS,
    GITHUB_API_BASE_URL,
    load_github_token,
)

if TYPE_CHECKING:
    from collections.abc import Sequence

GH_COMMAND = "gh"
GH_API_TIMEOUT_SECONDS = 30.0
HTTP_SUCCESS_STATUS_CODE = 200
HTTP_CREATED_STATUS_CODE = 201
HTTP_NO_CONTENT_STATUS_CODE = 204
GH_FAILURE_EXCEPTIONS: tuple[type[BaseException], ...] = (OSError, subprocess.TimeoutExpired)


def try_gh_api(*, endpoint: str, method: str, payload_text: str | None) -> str | None:
    """Run ``gh api`` once; None means gh failed, for any reason (ADR-0108).

    The command is list-form (no shell), so the endpoint string cannot
    inject shell metacharacters; it is still built from trusted inputs
    only (the parsed git remote slug). ``--input -`` carries the JSON
    request body on stdin when there is one. A zero exit means the
    call succeeded and stdout carries the response body (possibly
    empty); any other outcome (missing binary, auth, HTTP error,
    timeout) is None - the caller falls back to the REST API.
    """
    command = [GH_COMMAND, "api", endpoint, "--method", method]
    if payload_text is not None:
        command += ["--input", "-"]
    try:
        completed_process = subprocess.run(
            command,
            input=payload_text,
            capture_output=True,
            text=True,
            check=False,
            timeout=GH_API_TIMEOUT_SECONDS,
        )
    except GH_FAILURE_EXCEPTIONS:
        return None
    if completed_process.returncode != EXIT_CODE_SUCCESS:
        return None
    return completed_process.stdout


def parse_payload_text(*, payload_text: str) -> object | None:
    """Parse one gh response body; empty bodies parse to None, bad JSON fails closed."""
    stripped_text = payload_text.strip()
    if not stripped_text:
        return None
    try:
        payload: object = json.loads(stripped_text)
    except json.JSONDecodeError as decode_error:
        print(f"Error: undecodable response payload: {decode_error}", file=sys.stderr)
        raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE) from decode_error
    return payload


def rest_fallback(
    *, endpoint: str, method: str, payload_text: str | None
) -> tuple[int, object | None]:
    """One raw REST request with GITHUB_TOKEN, after gh failed (ADR-0108).

    The fallback needs the token; without it, the failure is closed
    with both transports named. A transport failure never passes as a
    GitHub answer: it fails the protocol.
    """
    github_token = load_github_token()
    try:
        response: httpx.Response = httpx.request(
            method,
            f"{GITHUB_API_BASE_URL}/{endpoint}",
            headers={
                "Authorization": f"Bearer {github_token}",
                "Accept": "application/vnd.github+json",
            },
            content=payload_text,
            timeout=GH_API_TIMEOUT_SECONDS,
        )
    except httpx.TransportError as transport_error:
        print(
            f"Error: gh failed and the REST fallback could not reach"
            f" {GITHUB_API_BASE_URL}: {transport_error}.",
            file=sys.stderr,
        )
        raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE) from transport_error
    response_body_text = response.text
    if not response_body_text.strip():
        return response.status_code, None
    try:
        payload: object = response.json()
    except json.JSONDecodeError:
        return response.status_code, None
    return response.status_code, payload


def github_request(
    *,
    endpoint: str,
    method: str = "GET",
    json_payload: object | None = None,
) -> tuple[int, object | None]:
    """One GitHub operation: gh first, REST fallback, fail closed (ADR-0108).

    Returns ``(status_code, payload)``. A successful gh call is
    reported with the 200 it carries (gh exits nonzero on any HTTP
    error, so a success can only be a success); any gh problem reruns
    the operation through one raw REST request, whose real status
    becomes the answer the caller branches on.
    """
    payload_text = json.dumps(json_payload) if json_payload is not None else None
    gh_output_text = try_gh_api(endpoint=endpoint, method=method, payload_text=payload_text)
    if gh_output_text is not None:
        return HTTP_SUCCESS_STATUS_CODE, parse_payload_text(payload_text=gh_output_text)
    return rest_fallback(endpoint=endpoint, method=method, payload_text=payload_text)


def fetch_pull_request(*, repository_slug: str, pull_request_number: int) -> dict[str, object]:
    """Fetch the pull request through gh (fallback REST), or fail closed."""
    status_code, payload = github_request(
        endpoint=f"repos/{repository_slug}/pulls/{pull_request_number}"
    )
    if status_code != HTTP_SUCCESS_STATUS_CODE or not isinstance(payload, dict):
        print(
            f"Error: could not fetch pull request #{pull_request_number}"
            f" through gh (status {status_code}).",
            file=sys.stderr,
        )
        raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE)
    return payload


def fetch_check_runs(*, repository_slug: str, head_sha: str) -> list[dict[str, object]]:
    """List the CI check runs of the pull request head, or fail closed."""
    status_code, payload = github_request(
        endpoint=f"repos/{repository_slug}/commits/{head_sha}/check-runs"
    )
    if status_code != HTTP_SUCCESS_STATUS_CODE or not isinstance(payload, dict):
        print(
            f"Error: could not fetch the check runs of {head_sha}"
            f" through gh (status {status_code}).",
            file=sys.stderr,
        )
        raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE)
    check_runs = payload.get("check_runs", [])
    if not isinstance(check_runs, list):
        return []
    return [entry for entry in check_runs if isinstance(entry, dict)]


def fetch_issue(*, repository_slug: str, item_number: int) -> dict[str, object]:
    """Fetch an issue or pull request (the issues endpoint serves both), or fail closed."""
    status_code, payload = github_request(endpoint=f"repos/{repository_slug}/issues/{item_number}")
    if status_code != HTTP_SUCCESS_STATUS_CODE or not isinstance(payload, dict):
        print(
            f"Error: could not fetch #{item_number} through gh (status {status_code}).",
            file=sys.stderr,
        )
        raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE)
    return payload


def extract_head_sha(*, pull_request: dict[str, object]) -> str:
    """Return the pull request head SHA, or fail closed."""
    head_container = pull_request.get("head")
    head_sha = head_container.get("sha", "") if isinstance(head_container, dict) else ""
    if not head_sha:
        print("Error: the pull request has no head SHA.", file=sys.stderr)
        raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE)
    return head_sha


def extract_head_ref(*, pull_request: dict[str, object]) -> str:
    """Return the pull request head branch name, empty when unknown."""
    head_container = pull_request.get("head")
    return head_container.get("ref", "") if isinstance(head_container, dict) else ""


def pull_request_has_merge_conflict(*, pull_request: dict[str, object]) -> bool:
    """Whether the pull request conflicts with its base branch.

    ``mergeable`` is computed asynchronously and may be missing right
    after a push: an unknown state is NOT a conflict. ``mergeable_state``
    carries a definitive ``dirty`` as soon as GitHub has computed it.
    A merge conflict is the first suspect whenever CI seems stuck
    (ADR-0108): the wait and merge drivers check it before waiting.
    """
    mergeable_state = pull_request.get("mergeable_state")
    if mergeable_state == "dirty":
        return True
    return pull_request.get("mergeable") is False


def check_runs_allow_merge(*, check_runs: Sequence[dict[str, object]]) -> str | None:
    """Verify every check run concluded success; a reason means: refused."""
    if len(check_runs) == 0:
        return "no CI check runs on the head - wait for CI, or push first"
    for check_run in check_runs:
        name = str(check_run.get("name", "unnamed check"))
        if check_run.get("status") != "completed":
            return f"CI check {name} is still running - wait for CI"
        if check_run.get("conclusion") != "success":
            return f"CI check {name} did not succeed - fix CI first"
    return None

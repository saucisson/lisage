"""Pin the gh-first GitHub client: gh first, REST fallback, fail closed (ADR-0108)."""

import json
from unittest.mock import MagicMock, patch

import httpx
import pytest

from scripts import _github
from scripts._shared import EXIT_CODE_PROTOCOL_FAILURE


def test_try_gh_api_returns_stdout_on_success() -> None:
    """A zero-exit gh call returns its stdout payload."""
    completed = MagicMock(returncode=0, stdout='{"ok":true}')
    with patch("subprocess.run", return_value=completed) as run_mock:
        result = _github.try_gh_api(endpoint="repos/o/r/pulls/1", method="GET", payload_text=None)
    assert result == '{"ok":true}'
    command = run_mock.call_args[0][0]
    assert command[:3] == ["gh", "api", "repos/o/r/pulls/1"]


def test_try_gh_api_returns_none_on_nonzero_exit() -> None:
    """A nonzero gh exit means failure - the caller falls back to REST."""
    completed = MagicMock(returncode=1, stdout="")
    with patch("subprocess.run", return_value=completed):
        result = _github.try_gh_api(endpoint="repos/o/r/pulls/1", method="GET", payload_text=None)
    assert result is None


def test_try_gh_api_returns_none_on_missing_binary() -> None:
    """gh not installed (OSError) means failure - fall back to REST."""
    with patch("subprocess.run", side_effect=OSError("no gh")):
        result = _github.try_gh_api(endpoint="repos/o/r/pulls/1", method="GET", payload_text=None)
    assert result is None


def test_try_gh_api_sends_payload_on_stdin() -> None:
    """A JSON payload travels on stdin via --input -."""
    completed = MagicMock(returncode=0, stdout="{}")
    with patch("subprocess.run", return_value=completed) as run_mock:
        _github.try_gh_api(endpoint="repos/o/r/pulls", method="POST", payload_text='{"title":"x"}')
    command = run_mock.call_args[0][0]
    assert "--input" in command
    assert "-" in command
    assert run_mock.call_args[1]["input"] == '{"title":"x"}'


def test_parse_payload_text_empty_is_none() -> None:
    """An empty gh body parses to None (204-style responses)."""
    assert _github.parse_payload_text(payload_text="  \n ") is None


def test_parse_payload_text_bad_json_fails_closed() -> None:
    """Undecodable JSON is a protocol failure, never a silent None."""
    with pytest.raises(SystemExit) as exc_info:
        _github.parse_payload_text(payload_text="not json{")
    assert exc_info.value.code == EXIT_CODE_PROTOCOL_FAILURE


def test_github_request_uses_gh_when_available() -> None:
    """When gh succeeds, the REST fallback is never touched."""
    with (
        patch.object(_github, "try_gh_api", return_value='{"number":42}'),
        patch.object(_github, "rest_fallback") as rest_mock,
    ):
        status, payload = _github.github_request(endpoint="repos/o/r/pulls/42")
    assert status == _github.HTTP_SUCCESS_STATUS_CODE
    assert payload == {"number": 42}
    rest_mock.assert_not_called()


def test_github_request_falls_back_to_rest() -> None:
    """When gh fails, exactly one REST request runs and its status is returned."""
    with (
        patch.object(_github, "try_gh_api", return_value=None),
        patch.object(
            _github, "rest_fallback", return_value=(404, {"message": "Not Found"})
        ) as rest_mock,
    ):
        status, payload = _github.github_request(endpoint="repos/o/r/pulls/999")
    assert status == 404
    assert payload == {"message": "Not Found"}
    rest_mock.assert_called_once()


def test_rest_fallback_builds_bearer_request() -> None:
    """The REST fallback sends Authorization: Bearer <token> to the API base."""
    response = httpx.Response(200, json={"ok": True})
    with (
        patch.object(_github, "load_github_token", return_value="secret-token"),
        patch.object(httpx, "request", return_value=response) as request_mock,
    ):
        status, payload = _github.rest_fallback(
            endpoint="repos/o/r", method="GET", payload_text=None
        )
    assert status == 200
    assert payload == {"ok": True}
    headers = request_mock.call_args[1]["headers"]
    assert headers["Authorization"] == "Bearer secret-token"


def test_rest_fallback_transport_error_fails_closed() -> None:
    """A network error on the fallback is a protocol failure, not a GitHub answer."""
    with (
        patch.object(_github, "load_github_token", return_value="secret-token"),
        patch.object(httpx, "request", side_effect=httpx.ConnectError("down")),
        pytest.raises(SystemExit) as exc_info,
    ):
        _github.rest_fallback(endpoint="repos/o/r", method="GET", payload_text=None)
    assert exc_info.value.code == EXIT_CODE_PROTOCOL_FAILURE


def test_pull_request_has_merge_conflict_dirty_state() -> None:
    """mergeable_state == 'dirty' is a definitive conflict."""
    pr = {"mergeable_state": "dirty", "mergeable": None}
    assert _github.pull_request_has_merge_conflict(pull_request=pr) is True


def test_pull_request_has_merge_conflict_mergeable_false() -> None:
    """mergeable == False is a conflict even without mergeable_state."""
    pr = {"mergeable": False}
    assert _github.pull_request_has_merge_conflict(pull_request=pr) is True


def test_pull_request_has_merge_conflict_unknown_is_not_conflict() -> None:
    """A missing mergeable (still computing) is NOT a conflict."""
    pr = {"mergeable": None, "mergeable_state": "unknown"}
    assert _github.pull_request_has_merge_conflict(pull_request=pr) is False


def test_check_runs_allow_merge_empty_refuses() -> None:
    """No check runs means: refused (wait for CI or push first)."""
    reason = _github.check_runs_allow_merge(check_runs=[])
    assert reason is not None
    assert "no CI check runs" in reason


def test_check_runs_allow_merge_all_success_allows() -> None:
    """Every completed+succeeded check run allows the merge."""
    runs = [
        {"name": "lint", "status": "completed", "conclusion": "success"},
        {"name": "test", "status": "completed", "conclusion": "success"},
    ]
    assert _github.check_runs_allow_merge(check_runs=runs) is None


def test_check_runs_allow_merge_failed_refuses() -> None:
    """A failed check run refuses the merge with the check named."""
    runs = [
        {"name": "lint", "status": "completed", "conclusion": "failure"},
    ]
    reason = _github.check_runs_allow_merge(check_runs=runs)
    assert reason is not None
    assert "lint" in reason


def test_fetch_pull_request_fails_closed_on_error() -> None:
    """A non-200 gh response for a PR fetch is a protocol failure."""
    with (
        patch.object(_github, "github_request", return_value=(404, {"message": "Not Found"})),
        pytest.raises(SystemExit) as exc_info,
    ):
        _github.fetch_pull_request(repository_slug="o/r", pull_request_number=999)
    assert exc_info.value.code == EXIT_CODE_PROTOCOL_FAILURE


def test_github_request_serializes_json_payload() -> None:
    """A json_payload dict is serialized to JSON text for gh stdin."""
    captured: dict[str, object] = {}

    def fake_try_gh_api(*, endpoint: str, method: str, payload_text: str | None) -> str:
        captured["payload_text"] = payload_text
        return "{}"

    with patch.object(_github, "try_gh_api", side_effect=fake_try_gh_api):
        _github.github_request(
            endpoint="repos/o/r/pulls", method="POST", json_payload={"title": "x"}
        )
    assert json.loads(captured["payload_text"]) == {"title": "x"}

"""Tests of the OIDC access-token validation brick (issue #309, level L1).

No network, no Docker, fully deterministic: the IdP is an httpx
MockTransport, the RSA keys are generated in fixtures, and the clock is
injected. Covers the acceptance criteria: valid, expired, wrong audience,
wrong issuer, bad signature, unknown kid (with and without rotation),
not-yet-valid, malformed and wrong-algorithm tokens.
"""

import base64
import json
import time
from typing import TYPE_CHECKING

import httpx
import pytest
from joserfc import jwt as jose_jwt
from joserfc.jwk import RSAKey

from notify.infrastructure.auth import AccessTokenVerifier, verify_access_token
from notify.models.auth import TokenValidationError
from notify.models.settings import AuthSettings
from notify.models.types import AccessToken, IssuerUrl

if TYPE_CHECKING:
    from structlog.typing import EventDict

ISSUER = IssuerUrl("https://auth.test.example")
CLIENT_ID = "lisage-client"
NOW = 1_800_000_000


def _b64url(data: bytes) -> str:
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode("ascii")


def _forge_token(header: dict[str, object], payload: dict[str, object]) -> AccessToken:
    """Build a compact JWS-looking string without a valid signature."""
    return AccessToken(
        f"{_b64url(json.dumps(header).encode())}.{_b64url(json.dumps(payload).encode())}.fakesig"
    )


def _sign(claims: dict[str, object], key: RSAKey, *, kid: str) -> AccessToken:
    return AccessToken(
        jose_jwt.encode(
            {"alg": "RS256", "kid": kid, "typ": "JWT"}, claims, key, algorithms=["RS256"]
        )
    )


def _valid_claims() -> dict[str, object]:
    return {
        "iss": ISSUER,
        "sub": "user-1",
        "aud": CLIENT_ID,
        "exp": NOW + 3600,
        "nbf": NOW - 10,
        "iat": NOW,
    }


class FakeIdp:
    """A scripted OIDC discovery + JWKS endpoint over httpx.MockTransport."""

    def __init__(self, keys: list[RSAKey]) -> None:
        self.keys = keys
        self.scripted_jwks: list[dict[str, object]] = []
        self.raw_jwks_responses: list[httpx.Response] = []
        self.raw_discovery_response: httpx.Response | None = None
        self.discovery_requests = 0
        self.jwks_requests = 0

    def handler(self, request: httpx.Request) -> httpx.Response:
        if request.url.path == "/.well-known/openid-configuration":
            self.discovery_requests += 1
            if self.raw_discovery_response is not None:
                return self.raw_discovery_response
            return httpx.Response(200, json={"issuer": ISSUER, "jwks_uri": f"{ISSUER}/jwks"})
        if request.url.path == "/jwks":
            self.jwks_requests += 1
            if self.raw_jwks_responses:
                return self.raw_jwks_responses.pop(0)
            if self.scripted_jwks:
                return httpx.Response(200, json=self.scripted_jwks.pop(0))
            return httpx.Response(200, json={"keys": [key.as_dict() for key in self.keys]})
        return httpx.Response(404)


@pytest.fixture
def signing_key() -> RSAKey:
    return RSAKey.generate_key(2048, parameters={"kid": "test-key-1"})


@pytest.fixture
def other_key() -> RSAKey:
    return RSAKey.generate_key(2048, parameters={"kid": "test-key-2"})


@pytest.fixture
def settings() -> AuthSettings:
    return AuthSettings(auth_issuer=ISSUER, auth_client_id=CLIENT_ID)


@pytest.fixture
def idp(signing_key: RSAKey) -> FakeIdp:
    return FakeIdp(keys=[signing_key])


def _make_verifier(settings: AuthSettings, idp: FakeIdp) -> AccessTokenVerifier:
    client = httpx.Client(transport=httpx.MockTransport(idp.handler))
    return AccessTokenVerifier(settings=settings, http_client=client, now=lambda: NOW)


def test_valid_token_returns_claims_and_caches_fetches(
    settings: AuthSettings, idp: FakeIdp, signing_key: RSAKey
) -> None:
    verifier = _make_verifier(settings, idp)
    token = _sign(_valid_claims(), signing_key, kid="test-key-1")
    claims = verifier.verify_access_token(token=token)
    assert claims.subject == "user-1"
    assert claims.issuer == ISSUER
    assert claims.audiences == [CLIENT_ID]
    assert claims.expires_at == NOW + 3600
    assert claims.not_before == NOW - 10
    assert claims.issued_at == NOW
    # A second verification reuses the cached discovery document and JWKS.
    verifier.verify_access_token(token=token)
    assert idp.discovery_requests == 1
    assert idp.jwks_requests == 1


def test_expired_token_rejected(settings: AuthSettings, idp: FakeIdp, signing_key: RSAKey) -> None:
    claims = _valid_claims()
    claims["exp"] = NOW - 120  # beyond the 60 s leeway
    token = _sign(claims, signing_key, kid="test-key-1")
    with pytest.raises(TokenValidationError) as excinfo:
        _make_verifier(settings, idp).verify_access_token(token=token)
    assert excinfo.value.reason == "expiration"


def test_wrong_audience_rejected(settings: AuthSettings, idp: FakeIdp, signing_key: RSAKey) -> None:
    claims = _valid_claims()
    claims["aud"] = "another-app"
    token = _sign(claims, signing_key, kid="test-key-1")
    with pytest.raises(TokenValidationError) as excinfo:
        _make_verifier(settings, idp).verify_access_token(token=token)
    assert excinfo.value.reason == "audience"


def test_wrong_issuer_rejected(settings: AuthSettings, idp: FakeIdp, signing_key: RSAKey) -> None:
    claims = _valid_claims()
    claims["iss"] = "https://evil.example"
    token = _sign(claims, signing_key, kid="test-key-1")
    with pytest.raises(TokenValidationError) as excinfo:
        _make_verifier(settings, idp).verify_access_token(token=token)
    assert excinfo.value.reason == "issuer"


def test_bad_signature_rejected(
    settings: AuthSettings, idp: FakeIdp, signing_key: RSAKey, other_key: RSAKey
) -> None:
    # Signed by a different private key while claiming kid test-key-1.
    token = _sign(_valid_claims(), other_key, kid="test-key-1")
    with pytest.raises(TokenValidationError) as excinfo:
        _make_verifier(settings, idp).verify_access_token(token=token)
    assert excinfo.value.reason == "signature"


def test_unknown_kid_triggers_refetch_and_succeeds_after_rotation(
    settings: AuthSettings, idp: FakeIdp, signing_key: RSAKey, other_key: RSAKey
) -> None:
    idp.scripted_jwks = [
        {"keys": [signing_key.as_dict()]},
        {"keys": [signing_key.as_dict(), other_key.as_dict()]},
    ]
    token = _sign(_valid_claims(), other_key, kid="test-key-2")
    claims = _make_verifier(settings, idp).verify_access_token(token=token)
    assert claims.subject == "user-1"
    assert idp.jwks_requests == 2


def test_unknown_kid_rejected_after_refetch(
    settings: AuthSettings, idp: FakeIdp, signing_key: RSAKey
) -> None:
    token = _sign(_valid_claims(), signing_key, kid="no-such-key")
    with pytest.raises(TokenValidationError) as excinfo:
        _make_verifier(settings, idp).verify_access_token(token=token)
    assert excinfo.value.reason == "unknown_key"
    assert idp.jwks_requests == 2  # initial fetch + one forced refresh


def test_unknown_kid_refresh_is_throttled(
    settings: AuthSettings, idp: FakeIdp, signing_key: RSAKey
) -> None:
    verifier = _make_verifier(settings, idp)
    for kid in ("no-such-key", "another-missing-key"):
        token = _sign(_valid_claims(), signing_key, kid=kid)
        with pytest.raises(TokenValidationError) as excinfo:
            verifier.verify_access_token(token=token)
        assert excinfo.value.reason == "unknown_key"
    # Two unknown kids, but only one forced refresh (60 s throttle).
    assert idp.jwks_requests == 2


def test_not_before_in_future_rejected(
    settings: AuthSettings, idp: FakeIdp, signing_key: RSAKey
) -> None:
    claims = _valid_claims()
    claims["nbf"] = NOW + 3600
    token = _sign(claims, signing_key, kid="test-key-1")
    with pytest.raises(TokenValidationError) as excinfo:
        _make_verifier(settings, idp).verify_access_token(token=token)
    assert excinfo.value.reason == "not_yet_valid"


def test_malformed_token_rejected(settings: AuthSettings, idp: FakeIdp) -> None:
    malformed = AccessToken("not-a-jwt")
    with pytest.raises(TokenValidationError) as excinfo:
        _make_verifier(settings, idp).verify_access_token(token=malformed)
    assert excinfo.value.reason == "malformed"


def test_wrong_algorithm_rejected(settings: AuthSettings, idp: FakeIdp) -> None:
    token = _forge_token({"alg": "HS256", "kid": "test-key-1"}, _valid_claims())
    with pytest.raises(TokenValidationError) as excinfo:
        _make_verifier(settings, idp).verify_access_token(token=token)
    assert excinfo.value.reason == "algorithm"


def test_debug_narration_events_are_pinned(
    settings: AuthSettings, idp: FakeIdp, signing_key: RSAKey, other_key: RSAKey,
    capture_logs: list[EventDict],
) -> None:
    """ADR-0005: the DEBUG narration events are pinned by this test —
    a rename or removal of one of them fails here."""
    idp.scripted_jwks = [
        {"keys": [signing_key.as_dict()]},
        {"keys": [signing_key.as_dict(), other_key.as_dict()]},
    ]
    verifier = _make_verifier(settings, idp)
    token = _sign(_valid_claims(), signing_key, kid="test-key-1")
    verifier.verify_access_token(token=token)
    # An unknown kid forces a JWKS refresh (rotation), then succeeds.
    rotated = _sign(_valid_claims(), other_key, kid="test-key-2")
    verifier.verify_access_token(token=rotated)

    events = [event["event"] for event in capture_logs]
    assert "oidc_discovery_fetched" in events
    assert events.count("jwks_fetched") == 2  # initial fetch + forced refresh
    assert "jwks_refresh_unknown_kid" in events
    discovery = next(e for e in capture_logs if e["event"] == "oidc_discovery_fetched")
    assert discovery["issuer"] == ISSUER
    refresh = next(e for e in capture_logs if e["event"] == "jwks_refresh_unknown_kid")
    assert refresh["kid"] == "test-key-2"


def test_rejection_is_logged_with_exact_reason(
    settings: AuthSettings, idp: FakeIdp, signing_key: RSAKey, capture_logs: list[EventDict]
) -> None:
    claims = _valid_claims()
    claims["aud"] = "another-app"
    token = _sign(claims, signing_key, kid="test-key-1")
    with pytest.raises(TokenValidationError):
        _make_verifier(settings, idp).verify_access_token(token=token)
    rejection_events = [
        event for event in capture_logs if event["event"] == "access_token_rejected"
    ]
    assert len(rejection_events) == 1
    assert rejection_events[0]["reason"] == "audience"
    assert token not in json.dumps(rejection_events)


def test_module_level_verify_access_token(
    settings: AuthSettings, idp: FakeIdp, signing_key: RSAKey
) -> None:
    # The module-level helper uses the real clock, so build claims around it.
    now = int(time.time())
    claims = _valid_claims()
    claims.update({"exp": now + 3600, "nbf": now - 10, "iat": now})
    client = httpx.Client(transport=httpx.MockTransport(idp.handler))
    token = _sign(claims, signing_key, kid="test-key-1")
    parsed = verify_access_token(token=token, settings=settings, http_client=client)
    assert parsed.subject == "user-1"


def test_accepted_audiences_default_to_client_id() -> None:
    settings = AuthSettings(auth_issuer=ISSUER, auth_client_id=CLIENT_ID)
    assert settings.accepted_audiences == [CLIENT_ID]
    # The comma-separated environment form is split by the field validator.
    explicit = AuthSettings.model_validate(
        {
            "auth_issuer": ISSUER,
            "auth_client_id": CLIENT_ID,
            "auth_allowed_audiences": "app-a, app-b",
        }
    )
    assert explicit.accepted_audiences == ["app-a", "app-b"]


def test_non_json_jwks_body_rejected_as_unavailable(
    settings: AuthSettings, idp: FakeIdp, signing_key: RSAKey
) -> None:
    idp.raw_jwks_responses = [httpx.Response(200, text="<html>not json</html>")]
    token = _sign(_valid_claims(), signing_key, kid="test-key-1")
    with pytest.raises(TokenValidationError) as excinfo:
        _make_verifier(settings, idp).verify_access_token(token=token)
    assert excinfo.value.reason == "jwks_unavailable"


def test_jwks_without_keys_list_rejected_as_unavailable(
    settings: AuthSettings, idp: FakeIdp, signing_key: RSAKey
) -> None:
    idp.raw_jwks_responses = [httpx.Response(200, json={"unexpected": "shape"})]
    token = _sign(_valid_claims(), signing_key, kid="test-key-1")
    with pytest.raises(TokenValidationError) as excinfo:
        _make_verifier(settings, idp).verify_access_token(token=token)
    assert excinfo.value.reason == "jwks_unavailable"


def test_jwks_with_unimportable_key_rejected_as_unavailable(
    settings: AuthSettings, idp: FakeIdp, signing_key: RSAKey
) -> None:
    # A key object missing its RSA parameters cannot be imported by joserfc.
    idp.raw_jwks_responses = [httpx.Response(200, json={"keys": [{"kty": "RSA", "kid": "x"}]})]
    token = _sign(_valid_claims(), signing_key, kid="test-key-1")
    with pytest.raises(TokenValidationError) as excinfo:
        _make_verifier(settings, idp).verify_access_token(token=token)
    assert excinfo.value.reason == "jwks_unavailable"


def test_non_json_discovery_body_rejected_as_unavailable(
    settings: AuthSettings, idp: FakeIdp, signing_key: RSAKey
) -> None:
    idp.raw_discovery_response = httpx.Response(200, text="not json at all")
    token = _sign(_valid_claims(), signing_key, kid="test-key-1")
    with pytest.raises(TokenValidationError) as excinfo:
        _make_verifier(settings, idp).verify_access_token(token=token)
    assert excinfo.value.reason == "jwks_unavailable"


def test_jwks_unavailable_rejection_is_logged(
    settings: AuthSettings,
    idp: FakeIdp,
    signing_key: RSAKey,
    capture_logs: list[EventDict],
) -> None:
    idp.raw_jwks_responses = [httpx.Response(200, text="<html>not json</html>")]
    token = _sign(_valid_claims(), signing_key, kid="test-key-1")
    with pytest.raises(TokenValidationError):
        _make_verifier(settings, idp).verify_access_token(token=token)
    rejection_events = [
        event for event in capture_logs if event["event"] == "access_token_rejected"
    ]
    assert len(rejection_events) == 1
    assert rejection_events[0]["reason"] == "jwks_unavailable"
    assert token not in json.dumps(rejection_events)


def test_explicit_allowed_audiences_accepts_listed_token(idp: FakeIdp, signing_key: RSAKey) -> None:
    settings = AuthSettings(
        auth_issuer=ISSUER, auth_client_id=CLIENT_ID, auth_allowed_audiences=["app-a", "app-b"]
    )
    claims = _valid_claims()
    claims["aud"] = ["app-b", "something-else"]
    token = _sign(claims, signing_key, kid="test-key-1")
    parsed = _make_verifier(settings, idp).verify_access_token(token=token)
    assert parsed.audiences == ["app-b", "something-else"]

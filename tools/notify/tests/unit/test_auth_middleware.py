"""Tests of the ASGI authentication middleware (issue #310, level L1).

No network, no server: the ASGI app is a fake capturing the scope, the
verifier is the real LS-1 brick with an httpx MockTransport IdP. Covers
the acceptance criteria: 401 on missing/malformed/invalid Bearer token,
claims exposed to the handler on success, pass-through when AUTH_DISABLED.
"""

import asyncio
import base64
import json

import httpx
import pytest
from joserfc import jwt as jose_jwt
from joserfc.jwk import RSAKey

from notify.infrastructure.auth.middleware import (
    SCOPE_CLAIMS_KEY,
    AuthenticationMiddleware,
    Message,
    Receive,
    Scope,
    Send,
    build_middleware,
)
from notify.infrastructure.auth.verify_access_token import AccessTokenVerifier
from notify.models.auth import AccessTokenClaims
from notify.models.settings import AuthSettings
from notify.models.types import AccessToken, IssuerUrl

ISSUER = IssuerUrl("https://auth.test.example")
API_CLIENT_ID = "lisage-api-client"
NOW = 1_800_000_000


def _b64url(data: bytes) -> str:
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode("ascii")


def _sign(claims: dict[str, object], key: RSAKey, *, kid: str) -> AccessToken:
    return AccessToken(
        jose_jwt.encode(
            {"alg": "RS256", "kid": kid, "typ": "JWT"}, claims, key, algorithms=["RS256"]
        )
    )


def _valid_claims() -> dict[str, object]:
    return {
        "iss": str(ISSUER),
        "sub": "user-1",
        "aud": API_CLIENT_ID,
        "exp": NOW + 3600,
        "nbf": NOW - 10,
        "iat": NOW,
    }


class _FakeIdp:
    """httpx MockTransport serving a discovery document and a JWKS."""

    def __init__(self, signing_key: RSAKey, kid: str) -> None:
        jwks = {"keys": [signing_key.as_dict(is_private=False)]}
        jwks["keys"][0]["kid"] = kid
        jwks["keys"][0]["alg"] = "RS256"
        jwks["keys"][0]["use"] = "sig"
        discovery = {
            "issuer": str(ISSUER),
            "jwks_uri": f"{ISSUER}/jwks.json",
        }

        def handler(request: httpx.Request) -> httpx.Response:
            path = request.url.path
            if path == "/.well-known/openid-configuration":
                return httpx.Response(200, json=discovery)
            if path == "/jwks.json":
                return httpx.Response(200, json=jwks)
            return httpx.Response(404)

        self.transport = httpx.MockTransport(handler)


@pytest.fixture()
def signing_key() -> RSAKey:
    return RSAKey.generate_key(2048, parameters={"kid": "key-1"})


@pytest.fixture()
def idp(signing_key: RSAKey) -> _FakeIdp:
    return _FakeIdp(signing_key, kid="key-1")


@pytest.fixture()
def settings() -> AuthSettings:
    return AuthSettings(
        auth_issuer=ISSUER,
        auth_api_client_id=API_CLIENT_ID,
        auth_disabled=False,
    )


@pytest.fixture()
def verifier(idp: _FakeIdp, settings: AuthSettings) -> AccessTokenVerifier:
    return AccessTokenVerifier(
        settings=settings,
        http_client=httpx.Client(transport=idp.transport),
        now=lambda: NOW,
    )


def _make_scope(*, authorization: str | None = None) -> Scope:
    headers: list[tuple[bytes, bytes]] = []
    if authorization is not None:
        headers.append((b"authorization", authorization.encode("latin-1")))
    return {
        "type": "http",
        "method": "GET",
        "path": "/",
        "headers": headers,
    }


async def _noop_app(scope: Scope, receive: Receive, send: Send) -> None:
    """A downstream ASGI app that does nothing."""


def _run_middleware(
    middleware: AuthenticationMiddleware,
    scope: Scope,
) -> tuple[list[Message], Scope | None]:
    """Run the middleware with a fake downstream app; return (sent messages, handler scope)."""
    sent: list[Message] = []
    handler_scope: Scope | None = None

    async def receive() -> Message:
        return {"type": "http.request", "body": b"", "more_body": False}

    async def send(message: Message) -> None:
        sent.append(message)

    async def downstream(app_scope: Scope, _receive: Receive, _send: Send) -> None:
        nonlocal handler_scope
        handler_scope = app_scope

    middleware._app = downstream
    asyncio.run(middleware(scope, receive, send))
    return sent, handler_scope


def test_missing_authorization_header_returns_401(verifier: AccessTokenVerifier) -> None:
    app = AuthenticationMiddleware(app=_noop_app, verifier=verifier)
    sent, handler_scope = _run_middleware(app, _make_scope())
    assert handler_scope is None
    assert sent[0]["type"] == "http.response.start"
    assert sent[0]["status"] == 401
    body_bytes = sent[1]["body"]
    assert isinstance(body_bytes, bytes)
    body = json.loads(body_bytes)
    assert body["error"]["code"] == "unauthorized"
    assert body["error"]["reason"] == "malformed"


def test_non_bearer_scheme_returns_401(verifier: AccessTokenVerifier) -> None:
    app = AuthenticationMiddleware(app=_noop_app, verifier=verifier)
    sent, handler_scope = _run_middleware(app, _make_scope(authorization="Basic abc"))
    assert handler_scope is None
    assert sent[0]["status"] == 401


def test_invalid_token_returns_401_with_reason(
    verifier: AccessTokenVerifier, signing_key: RSAKey
) -> None:
    bad_token = _sign({**_valid_claims(), "exp": NOW - 120}, signing_key, kid="key-1")
    app = AuthenticationMiddleware(app=_noop_app, verifier=verifier)
    sent, handler_scope = _run_middleware(app, _make_scope(authorization=f"Bearer {bad_token}"))
    assert handler_scope is None
    assert sent[0]["status"] == 401
    body_bytes = sent[1]["body"]
    assert isinstance(body_bytes, bytes)
    body = json.loads(body_bytes)
    assert body["error"]["reason"] == "expiration"


def test_valid_token_exposes_claims_to_handler(
    verifier: AccessTokenVerifier, signing_key: RSAKey
) -> None:
    token = _sign(_valid_claims(), signing_key, kid="key-1")
    app = AuthenticationMiddleware(app=_noop_app, verifier=verifier)
    sent, handler_scope = _run_middleware(app, _make_scope(authorization=f"Bearer {token}"))
    assert handler_scope is not None
    claims = handler_scope[SCOPE_CLAIMS_KEY]
    assert isinstance(claims, AccessTokenClaims)
    assert claims.subject == "user-1"
    assert str(claims.issuer) == str(ISSUER)
    # No 401 was sent: the downstream app ran (no response messages captured).
    assert sent == []


def test_auth_disabled_passes_through(verifier: AccessTokenVerifier) -> None:
    app = AuthenticationMiddleware(app=_noop_app, verifier=verifier, auth_disabled=True)
    sent, handler_scope = _run_middleware(app, _make_scope())
    assert handler_scope is not None
    assert SCOPE_CLAIMS_KEY not in handler_scope
    assert sent == []


def test_non_http_scope_passes_through(verifier: AccessTokenVerifier) -> None:
    app = AuthenticationMiddleware(app=_noop_app, verifier=verifier)
    scope: Scope = {"type": "websocket", "headers": []}
    sent, handler_scope = _run_middleware(app, scope)
    assert handler_scope is not None
    assert sent == []


def test_build_middleware_from_settings(idp: _FakeIdp, settings: AuthSettings) -> None:
    middleware = build_middleware(
        app=_noop_app,
        settings=settings,
        http_client=httpx.Client(transport=idp.transport),
    )
    assert isinstance(middleware, AuthenticationMiddleware)
    assert middleware._auth_disabled is False


def test_build_middleware_auth_disabled(idp: _FakeIdp) -> None:
    settings = AuthSettings(
        auth_issuer=ISSUER,
        auth_api_client_id=API_CLIENT_ID,
        auth_disabled=True,
    )
    middleware = build_middleware(
        app=_noop_app,
        settings=settings,
        http_client=httpx.Client(transport=idp.transport),
    )
    assert middleware._auth_disabled is True

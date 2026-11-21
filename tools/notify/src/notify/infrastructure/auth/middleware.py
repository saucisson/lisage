"""ASGI authentication middleware: Bearer-token gate for future HTTP routes (issue #310).

No web framework is required: this is a pure ASGI middleware, compatible
with any ASGI server or framework (Starlette, FastAPI, raw ASGI). It
extracts the ``Authorization: Bearer <token>`` header, validates the token
with the LS-1 brick (:class:`AccessTokenVerifier`) and exposes the validated
claims to downstream handlers through ``scope["auth_claims"]``.

Rejections answer ``401 Unauthorized`` with a typed JSON error payload
(ADR-0016): ``{"error": {"code": "unauthorized", "reason": <reason>}}``.
No token material is ever logged or returned.
"""

import json
from collections.abc import Awaitable, Callable, MutableMapping
from typing import TYPE_CHECKING

import httpx
import structlog

from notify.infrastructure.auth.verify_access_token import AccessTokenVerifier
from notify.models.auth import TokenValidationError
from notify.models.types import AccessToken

if TYPE_CHECKING:
    from notify.models.settings import AuthSettings

logger = structlog.get_logger(__name__)

# ASGI type aliases (PEP 3333): a message is a mutable mapping of str keys
# to JSON-like values; no bare ``Any`` at the boundary (ADR-0006).
Message = MutableMapping[str, object]
Scope = MutableMapping[str, object]
Receive = Callable[[], Awaitable[Message]]
Send = Callable[[Message], Awaitable[None]]
ASGIApp = Callable[[Scope, Receive, Send], Awaitable[None]]

AUTHORIZATION_HEADER = "authorization"
BEARER_SCHEME = "bearer"
BEARER_PARTS_COUNT = 2
SCOPE_CLAIMS_KEY = "auth_claims"
SCOPE_TYPE_HTTP = "http"
UNAUTHORIZED_STATUS = 401
ERROR_CONTENT_TYPE = "application/json"


class AuthenticationMiddleware:
    """Pure ASGI middleware enforcing a valid Bearer access token.

    When ``auth_disabled`` is set (local development only), the middleware
    is a pass-through and no claims are attached.
    """

    def __init__(
        self,
        *,
        app: ASGIApp,
        verifier: AccessTokenVerifier,
        auth_disabled: bool = False,
    ) -> None:
        """Hold the downstream app, the token verifier and the disabled flag."""
        self._app = app
        self._verifier = verifier
        self._auth_disabled = auth_disabled

    async def __call__(self, scope: Scope, receive: Receive, send: Send) -> None:
        """Gate the request: validate the Bearer token or answer 401."""
        if scope.get("type") != SCOPE_TYPE_HTTP:
            await self._app(scope, receive, send)
            return
        if self._auth_disabled:
            await self._app(scope, receive, send)
            return

        token = _extract_bearer_token(scope=scope)
        if token is None:
            await _send_unauthorized(send=send, reason="malformed")
            return

        try:
            claims = self._verifier.verify_access_token(token=token)
        except TokenValidationError as error:
            logger.warning("access_token_rejected", reason=error.reason)
            await _send_unauthorized(send=send, reason=error.reason)
            return

        scope[SCOPE_CLAIMS_KEY] = claims
        await self._app(scope, receive, send)


def _extract_bearer_token(*, scope: Scope) -> AccessToken | None:
    """Extract the Bearer token from the Authorization header, or None."""
    headers = scope.get("headers", [])
    if not isinstance(headers, list):
        return None
    for name, value in headers:
        if not isinstance(name, bytes) or not isinstance(value, bytes):
            continue
        if name.decode("latin-1").lower() == AUTHORIZATION_HEADER:
            header_value = value.decode("latin-1")
            parts = header_value.split(None, 1)
            if len(parts) == BEARER_PARTS_COUNT and parts[0].lower() == BEARER_SCHEME:
                return AccessToken(parts[1].strip())
            return None
    return None


async def _send_unauthorized(*, send: Send, reason: str) -> None:
    """Answer 401 with a typed JSON error payload (ADR-0016)."""
    body = json.dumps({"error": {"code": "unauthorized", "reason": reason}}).encode("utf-8")
    await send(
        {
            "type": "http.response.start",
            "status": UNAUTHORIZED_STATUS,
            "headers": [
                (b"content-type", ERROR_CONTENT_TYPE.encode("latin-1")),
                (b"content-length", str(len(body)).encode("latin-1")),
            ],
        }
    )
    await send({"type": "http.response.body", "body": body})


def build_middleware(
    *,
    app: ASGIApp,
    settings: AuthSettings,
    http_client: httpx.Client | None = None,
) -> AuthenticationMiddleware:
    """Wire the middleware from settings: build the verifier, honor AUTH_DISABLED."""
    client = http_client if http_client is not None else httpx.Client()
    verifier = AccessTokenVerifier(settings=settings, http_client=client)
    return AuthenticationMiddleware(
        app=app,
        verifier=verifier,
        auth_disabled=settings.auth_disabled,
    )

"""Local validation of OIDC access tokens (RS256 JWT) against the IdP JWKS.

Brick of the unified authentication (issue #309, cadrage "Authentification
unifiée saucisson"): the discovery document and the JWKS are fetched over
HTTP and cached (discovery 24 h, JWKS 1 h, forced refresh on an unknown
``kid``); the signature is verified with joserfc and the claims (``iss``,
``aud``, ``exp``, ``nbf``) are checked with a 60 s clock-skew leeway. No
endpoint URL is hard-coded: everything derives from ``AUTH_ISSUER``.

Rejections raise :class:`TokenValidationError` with an exact, loggable
reason; no token material is ever logged.
"""

import base64
import binascii
import json
import time
from typing import TYPE_CHECKING, NoReturn, cast

import httpx
import structlog
from joserfc import jwt as jose_jwt
from joserfc.errors import (
    BadSignatureError,
    ClaimError,
    ExpiredTokenError,
    InvalidKeyIdError,
    JoseError,
)
from joserfc.jwk import KeySet, KeySetSerialization
from joserfc.jwt import Claims, JWTClaimsRegistry

from notify.models.auth import AccessTokenClaims, TokenRejectionReason, TokenValidationError
from notify.models.types import IssuerUrl

if TYPE_CHECKING:
    from collections.abc import Callable

    from notify.models.settings import AuthSettings
    from notify.models.types import AccessToken

logger = structlog.get_logger(__name__)

DISCOVERY_CACHE_TTL_SECONDS = 24 * 60 * 60
JWKS_CACHE_TTL_SECONDS = 60 * 60
# Minimum delay between two forced JWKS refreshes: an attacker spraying
# unknown kids must not turn each token into one IdP HTTP request.
JWKS_REFRESH_THROTTLE_SECONDS = 60
CLOCK_SKEW_LEEWAY_SECONDS = 60
HTTP_TIMEOUT_SECONDS = 10.0
REQUIRED_SIGNING_ALGORITHM = "RS256"
OIDC_DISCOVERY_PATH = "/.well-known/openid-configuration"
COMPACT_JWS_SEGMENT_COUNT = 3

_CLAIM_REASONS: dict[str, TokenRejectionReason] = {
    "iss": "issuer",
    "aud": "audience",
    "exp": "expiration",
    "nbf": "not_yet_valid",
}


def _system_now() -> int:
    return int(time.time())


def _reason_for_claim(*, claim: str) -> TokenRejectionReason:
    """Map a failing claim name to its rejection reason (missing keys join with ',')."""
    for name in claim.split(","):
        reason = _CLAIM_REASONS.get(name.strip())
        if reason is not None:
            return reason
    return "malformed"


class AccessTokenVerifier:
    """Validates OIDC access tokens, caching the discovery document and the JWKS."""

    def __init__(
        self,
        *,
        settings: AuthSettings,
        http_client: httpx.Client,
        now: Callable[[], int] = _system_now,
    ) -> None:
        """Hold the settings and the HTTP client; caches start empty."""
        self._settings = settings
        self._http_client = http_client
        self._now = now
        self._discovery: tuple[int, dict[str, object]] | None = None
        self._jwks: tuple[int, KeySet] | None = None
        self._last_forced_refresh: int | None = None

    def verify_access_token(self, *, token: AccessToken) -> AccessTokenClaims:
        """Verify one access token and return its validated claims.

        :raises TokenValidationError: with the exact rejection reason.
        """
        header = self._decode_header(token=token)
        algorithm = header.get("alg")
        if algorithm != REQUIRED_SIGNING_ALGORITHM:
            self._reject(reason="algorithm", detail=f"alg={algorithm!r}")
        kid = header.get("kid")
        if not isinstance(kid, str) or not kid:
            self._reject(reason="malformed", detail="missing kid header")
        key_set = self._key_set_for(kid=kid)
        try:
            verified = jose_jwt.decode(token, key_set, algorithms=[REQUIRED_SIGNING_ALGORITHM])
        except BadSignatureError:
            self._reject(reason="signature", detail="bad signature")
        except JoseError as error:
            self._reject(reason="malformed", detail=type(error).__name__)
        claims = verified.claims
        self._validate_claims(claims=claims)
        return self._to_domain_claims(claims=claims)

    def _reject(self, *, reason: TokenRejectionReason, detail: str) -> NoReturn:
        logger.warning("access_token_rejected", reason=reason, detail=detail)
        raise TokenValidationError(
            reason=reason, message=f"access token rejected: {reason} ({detail})"
        )

    def _decode_header(self, *, token: AccessToken) -> dict[str, object]:
        parts = token.split(".")
        if len(parts) != COMPACT_JWS_SEGMENT_COUNT:
            self._reject(reason="malformed", detail="not a compact JWS")
        try:
            padded = parts[0] + "=" * (-len(parts[0]) % 4)
            header: object = json.loads(base64.urlsafe_b64decode(padded))
        except (binascii.Error, ValueError) as error:
            self._reject(reason="malformed", detail=type(error).__name__)
        if not isinstance(header, dict):
            self._reject(reason="malformed", detail="header is not a JSON object")
        return header

    def _key_set_for(self, *, kid: str) -> KeySet:
        key_set = self._cached_jwks()
        if self._has_kid(key_set=key_set, kid=kid):
            return key_set
        now = self._now()
        if (
            self._last_forced_refresh is not None
            and now - self._last_forced_refresh < JWKS_REFRESH_THROTTLE_SECONDS
        ):
            self._reject(
                reason="unknown_key",
                detail=f"kid={kid!r} unknown, refresh throttled",
            )
        logger.debug("jwks_refresh_unknown_kid", kid=kid)
        self._last_forced_refresh = now
        refreshed = self._fetch_jwks(force=True)
        if not self._has_kid(key_set=refreshed, kid=kid):
            self._reject(reason="unknown_key", detail=f"kid={kid!r} absent from JWKS")
        return refreshed

    @staticmethod
    def _has_kid(*, key_set: KeySet, kid: str) -> bool:
        try:
            key_set.get_by_kid(kid)
        except InvalidKeyIdError:
            return False
        return True

    def _cached_jwks(self) -> KeySet:
        now = self._now()
        if self._jwks is not None and now - self._jwks[0] < JWKS_CACHE_TTL_SECONDS:
            return self._jwks[1]
        return self._fetch_jwks(force=False)

    def _fetch_jwks(self, *, force: bool) -> KeySet:
        now = self._now()
        if not force and self._jwks is not None and now - self._jwks[0] < JWKS_CACHE_TTL_SECONDS:
            return self._jwks[1]
        jwks_uri = self._jwks_uri()
        try:
            response = self._http_client.get(jwks_uri, timeout=HTTP_TIMEOUT_SECONDS)
            response.raise_for_status()
        except httpx.HTTPError as error:
            self._reject(
                reason="jwks_unavailable",
                detail=f"JWKS fetch failed: {type(error).__name__}",
            )
        try:
            payload: object = response.json()
        except ValueError as error:  # json.JSONDecodeError: IdP answered 200 with non-JSON
            self._reject(
                reason="jwks_unavailable", detail=f"JWKS is not JSON: {type(error).__name__}"
            )
        if not isinstance(payload, dict):
            self._reject(reason="jwks_unavailable", detail="JWKS is not a JSON object")
        keys = payload.get("keys")
        if not isinstance(keys, list) or not all(isinstance(key, dict) for key in keys):
            self._reject(
                reason="jwks_unavailable", detail="JWKS has no list-of-objects keys member"
            )
        try:
            # Runtime-verified above: a dict whose "keys" is a list of objects is
            # exactly the KeySetSerialization shape joserfc expects (RFC 7517).
            key_set = KeySet.import_key_set(cast("KeySetSerialization", payload))
        except JoseError as error:
            self._reject(
                reason="jwks_unavailable",
                detail=f"JWKS keys not importable: {type(error).__name__}",
            )
        self._jwks = (now, key_set)
        logger.debug("jwks_fetched", jwks_uri=jwks_uri, key_count=len(key_set.keys))
        return key_set

    def _jwks_uri(self) -> str:
        now = self._now()
        if self._discovery is not None and now - self._discovery[0] < DISCOVERY_CACHE_TTL_SECONDS:
            document = self._discovery[1]
        else:
            document = self._fetch_discovery()
        jwks_uri = document.get("jwks_uri")
        if not isinstance(jwks_uri, str) or not jwks_uri:
            self._reject(reason="jwks_unavailable", detail="discovery document has no jwks_uri")
        return jwks_uri

    def _fetch_discovery(self) -> dict[str, object]:
        url = self._settings.auth_issuer.rstrip("/") + OIDC_DISCOVERY_PATH
        try:
            response = self._http_client.get(url, timeout=HTTP_TIMEOUT_SECONDS)
            response.raise_for_status()
        except httpx.HTTPError as error:
            self._reject(
                reason="jwks_unavailable",
                detail=f"discovery fetch failed: {type(error).__name__}",
            )
        try:
            document: object = response.json()
        except ValueError as error:  # json.JSONDecodeError: IdP answered 200 with a non-JSON body
            self._reject(
                reason="jwks_unavailable",
                detail=f"discovery document is not JSON: {type(error).__name__}",
            )
        if not isinstance(document, dict):
            self._reject(
                reason="jwks_unavailable", detail="discovery document is not a JSON object"
            )
        self._discovery = (self._now(), document)
        logger.debug("oidc_discovery_fetched", issuer=self._settings.auth_issuer)
        return document

    def _validate_claims(self, *, claims: Claims) -> None:
        settings = self._settings
        registry = JWTClaimsRegistry(
            now=self._now,
            leeway=CLOCK_SKEW_LEEWAY_SECONDS,
            iss={"essential": True, "value": settings.auth_issuer},
            aud={"essential": True, "values": settings.accepted_audiences},
            exp={"essential": True},
            nbf={"essential": False},
        )
        try:
            registry.validate(claims)
        except ExpiredTokenError:
            self._reject(reason="expiration", detail="token expired")
        except ClaimError as error:
            self._reject(reason=_reason_for_claim(claim=error.claim), detail=type(error).__name__)

    def _to_domain_claims(self, *, claims: Claims) -> AccessTokenClaims:
        subject = claims.get("sub")
        issuer = claims.get("iss")
        expires_at = claims.get("exp")
        if (
            not isinstance(subject, str)
            or not isinstance(issuer, str)
            or not isinstance(expires_at, int)
        ):
            self._reject(reason="malformed", detail="missing or ill-typed sub/iss/exp")
        raw_audiences = claims.get("aud")
        if isinstance(raw_audiences, str):
            audiences = [raw_audiences]
        elif isinstance(raw_audiences, list) and all(
            isinstance(item, str) for item in raw_audiences
        ):
            audiences = [item for item in raw_audiences if isinstance(item, str)]
        else:
            self._reject(reason="malformed", detail="aud is not a string or string list")
        not_before = claims.get("nbf")
        issued_at = claims.get("iat")
        return AccessTokenClaims(
            subject=subject,
            issuer=IssuerUrl(issuer),
            audiences=audiences,
            expires_at=expires_at,
            not_before=not_before if isinstance(not_before, int) else None,
            issued_at=issued_at if isinstance(issued_at, int) else None,
        )


def verify_access_token(
    *, token: AccessToken, settings: AuthSettings, http_client: httpx.Client
) -> AccessTokenClaims:
    """One-shot access-token validation.

    Long-lived callers should reuse one :class:`AccessTokenVerifier` so the
    discovery and JWKS caches are shared across calls.
    """
    verifier = AccessTokenVerifier(settings=settings, http_client=http_client)
    return verifier.verify_access_token(token=token)

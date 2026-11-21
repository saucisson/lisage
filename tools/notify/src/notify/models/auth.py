"""Domain types of the unified authentication brick (Zitadel OIDC, issue #309).

Access tokens are RS256-signed JWTs validated locally against the IdP's
JWKS. This module holds the validated claims and the typed rejection error;
no password, e-mail or login URL ever lives here - everything comes from
the identity provider.
"""

from typing import Literal

from pydantic import BaseModel

from notify.models.errors import NotifyError

#: Machine-checkable cause of a token rejection (logged, never secret).
TokenRejectionReason = Literal[
    "malformed",
    "algorithm",
    "unknown_key",
    "jwks_unavailable",
    "signature",
    "issuer",
    "audience",
    "expiration",
    "not_yet_valid",
]


class TokenValidationError(NotifyError):
    """An access token was rejected; ``reason`` is the exact cause."""

    def __init__(self, *, reason: TokenRejectionReason, message: str) -> None:
        """Carry the machine-checkable rejection reason alongside the message."""
        self.reason: TokenRejectionReason = reason
        super().__init__(message)


class AccessTokenClaims(BaseModel):
    """The validated claims of an OIDC access token."""

    subject: str
    issuer: str
    audiences: list[str]
    expires_at: int
    not_before: int | None = None
    issued_at: int | None = None

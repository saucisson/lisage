"""Property tests for the auth brick invariants (ADR-0012, issue #309).

Hypothesis falsifies two invariants that single examples cannot prove:
the comma-separated ``AUTH_ALLOWED_AUDIENCES`` parsing (whatever the raw
string, the accepted list is its stripped, non-empty parts - or the
client id when nothing survives), and the claim-to-rejection-reason
mapping (any audience outside the accepted set is rejected as
``audience``, whatever its shape).
"""

import httpx
import pytest
from hypothesis import assume, given
from hypothesis import settings as hypothesis_settings
from hypothesis import strategies as st
from joserfc import jwt as jose_jwt
from joserfc.jwk import RSAKey

from notify.infrastructure.auth import AccessTokenVerifier
from notify.models.auth import TokenValidationError
from notify.models.settings import AuthSettings
from notify.models.types import AccessToken, IssuerUrl

pytestmark = pytest.mark.property

ISSUER = IssuerUrl("https://auth.test.example")
CLIENT_ID = "lisage-client"
NOW = 1_800_000_000

# Printable ASCII only: lone surrogates would break JSON serialization.
SAFE_TEXT = st.text(alphabet=st.characters(blacklist_categories=("Cs", "Cc")), max_size=60)

_signing_key = RSAKey.generate_key(2048, parameters={"kid": "property-key"})


def _idp_handler(request: httpx.Request) -> httpx.Response:
    if request.url.path == "/.well-known/openid-configuration":
        return httpx.Response(200, json={"issuer": ISSUER, "jwks_uri": f"{ISSUER}/jwks"})
    if request.url.path == "/jwks":
        return httpx.Response(200, json={"keys": [_signing_key.as_dict()]})
    return httpx.Response(404)


def _sign(claims: dict[str, object]) -> AccessToken:
    return AccessToken(
        jose_jwt.encode(
            {"alg": "RS256", "kid": "property-key", "typ": "JWT"},
            claims,
            _signing_key,
            algorithms=["RS256"],
        )
    )


@hypothesis_settings(max_examples=200)
@given(raw_audiences=SAFE_TEXT)
def test_allowed_audiences_parsing_holds_for_any_raw_string(raw_audiences: str) -> None:
    """Whatever the env string, accepted audiences are its stripped non-empty parts."""
    settings = AuthSettings.model_validate(
        {
            "auth_issuer": ISSUER,
            "auth_client_id": CLIENT_ID,
            "auth_allowed_audiences": raw_audiences,
        }
    )
    expected = [item.strip() for item in raw_audiences.split(",") if item.strip()]
    assert settings.accepted_audiences == (expected if expected else [CLIENT_ID])


@hypothesis_settings(max_examples=100)
@given(foreign_audience=SAFE_TEXT)
def test_any_foreign_audience_is_rejected_as_audience(foreign_audience: str) -> None:
    """Any aud outside the accepted set maps to the ``audience`` rejection reason."""
    assume(foreign_audience != CLIENT_ID)
    settings = AuthSettings(auth_issuer=ISSUER, auth_client_id=CLIENT_ID)
    client = httpx.Client(transport=httpx.MockTransport(_idp_handler))
    verifier = AccessTokenVerifier(settings=settings, http_client=client, now=lambda: NOW)
    token = _sign(
        {
            "iss": ISSUER,
            "sub": "user-1",
            "aud": foreign_audience,
            "exp": NOW + 3600,
            "nbf": NOW - 10,
            "iat": NOW,
        }
    )
    with pytest.raises(TokenValidationError) as excinfo:
        verifier.verify_access_token(token=token)
    assert excinfo.value.reason == "audience"

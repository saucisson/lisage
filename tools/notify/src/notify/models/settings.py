"""Application settings, loaded from the environment.

Secrets arrive as ``SecretStr`` so no log call can leak them by accident
(ADR-0005, ADR-0008). Every field is optional-friendly: the application
must start in the barest possible environment (console fallback).
"""

from enum import StrEnum
from typing import Literal

from pydantic import SecretStr, field_validator
from pydantic_settings import BaseSettings, SettingsConfigDict

from notify.models.types import IssuerUrl

LogLevel = Literal["DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"]


class LogFormat(StrEnum):
    """Rendering of log events: console for humans, JSON for machines."""

    CONSOLE = "console"
    JSON = "json"


class ApplicationSettings(BaseSettings):
    """Environment-driven configuration, validated at startup."""

    model_config = SettingsConfigDict(
        env_prefix="",
        # Case-insensitive matching is what makes the documented uppercase
        # variables (DISCORD_WEBHOOK_URL, LOG_LEVEL, LOG_FORMAT) load: field
        # names are lowercase, and pydantic-settings matches them verbatim
        # against the environment when case_sensitive=True.
        case_sensitive=False,
        frozen=True,
    )

    discord_webhook_url: SecretStr | None = None
    log_level: LogLevel = "INFO"
    log_format: LogFormat = LogFormat.CONSOLE


class AuthSettings(BaseSettings):
    """OIDC configuration of this app, loaded from the environment (issue #309).

    Zitadel exposes two clients for the app: the **API** client (machine
    access, Bearer/PAT tokens) and the **Web** client (browser
    Authorization Code + PKCE flow). Their ids and secrets arrive as
    ``AUTH_API_CLIENT_ID`` / ``AUTH_API_CLIENT_SECRET`` and
    ``AUTH_WEB_CLIENT_ID`` / ``AUTH_WEB_CLIENT_SECRET``; on the VPS they
    live in ``/home/deploy/zitadel-clients.json`` and are injected into the
    environment at deploy time - never committed.

    No endpoint URL is hard-coded: the issuer (``AUTH_ISSUER``, required)
    drives the discovery document, and the JWKS URI comes from it.
    ``AUTH_DISABLED=1`` is a local-development escape hatch only -
    production deployments leave it unset so authentication stays active.
    """

    model_config = SettingsConfigDict(
        env_prefix="",
        # Same case-insensitive matching as ApplicationSettings: the
        # documented uppercase variables (AUTH_ISSUER, ...) load verbatim.
        case_sensitive=False,
        frozen=True,
    )

    auth_issuer: IssuerUrl
    auth_api_client_id: str | None = None
    auth_api_client_secret: SecretStr | None = None
    auth_web_client_id: str | None = None
    auth_web_client_secret: SecretStr | None = None
    auth_allowed_audiences: list[str] = []
    auth_disabled: bool = False

    @field_validator("auth_allowed_audiences", mode="before")
    @classmethod
    def _split_comma_separated(cls, value: object) -> object:
        if isinstance(value, str):
            return [item.strip() for item in value.split(",") if item.strip()]
        return value

    @property
    def accepted_audiences(self) -> list[str]:
        """Audiences accepted in access tokens.

        The explicit ``AUTH_ALLOWED_AUDIENCES`` list wins; otherwise both
        client ids (API and Web) are accepted so Bearer tokens issued to
        either client validate.
        """
        if self.auth_allowed_audiences:
            return list(self.auth_allowed_audiences)
        audiences: list[str] = []
        if self.auth_api_client_id is not None:
            audiences.append(self.auth_api_client_id)
        if self.auth_web_client_id is not None:
            audiences.append(self.auth_web_client_id)
        return audiences

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
    auth_client_id: str | None = None
    auth_client_secret: SecretStr | None = None
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
        """Audiences accepted in access tokens: the explicit list, else the client id."""
        if self.auth_allowed_audiences:
            return list(self.auth_allowed_audiences)
        if self.auth_client_id is not None:
            return [self.auth_client_id]
        return []

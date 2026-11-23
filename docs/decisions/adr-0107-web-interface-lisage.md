# ADR-0107: Web surface for lisage — lisage.linard.fr (three-zone model, unified saucisson auth)

- Status: draft (drafted 2026-11-21; to be accepted by the human owner after review)
- Date: 2026-11-21

> Draft written alongside the WEB milestone issues (#330-#335). A project ADR
> is *drafted* in a PR (numbered from 0100 upward); only the human accepts it.
> This ADR is to move from **draft** to **accepted** once the human reviews the
> unified saucisson web contract (`docs/lisage-web-v1/consigne-interface-web.md`).

## Context

The lisage project needs a public web surface at `lisage.linard.fr`. Three
saucisson sites must converge on the same model — `layered-data.linard.fr`,
`petrichor.linard.fr`, `lisage.linard.fr` — each with a public landing page, a
public discovery area, and an authenticated workshop. The convergence is by
contract, not by shared package: each repository stays autonomous.

Authentication is exclusively through `auth.linard.fr` (Zitadel IdP, already
deployed and operational); no local authentication remains. The unified
contract is documented in `docs/lisage-web-v1/consigne-interface-web.md`
(delivered 2026-11-23 as `lisage-web-v1.zip`).

The functional framing (crate `lisage-web`, gallery, workshop API, deployment)
was set on 2026-11-21 in the WEB milestone (issues #330-#335). The 2026-11-23
contract adds the common authentication, zone, and UI requirements on top.

## Decision

1. **Three-zone model** — landing (`/`, public), discovery/gallery (public),
   workshop (authenticated). Unauthenticated visitors on the workshop get an
   explicit message + single "Se connecter avec auth.linard.fr" button; server
   routes without session return `401` in `application/problem+json` with
   `reason: "session_required"`.

2. **Authentication via `auth.linard.fr` only** — Authorization Code + PKCE
   S256 flow driven by axum in `lisage-web` (browser never sees an OIDC token):
   `GET /auth/login?returnTo=…`, `GET /auth/callback`, `POST /auth/logout`.
   App session: cookie `lisage-session` (`httpOnly`, `Secure`, `SameSite=Lax`,
   bounded lifetime, rotation on login), in-memory server storage. Machine
   access: Zitadel PAT (`Authorization: Bearer`) accepted on workshop routes in
   addition to the cookie — Rust equivalent of the notify token-validation
   brick (local JWT RS256 validation + JWKS + introspection). `AUTH_DISABLED=1`
   dev-only guard, disabled by default, test fails if active outside dev.
   Environment variables: `AUTH_ISSUER`, `AUTH_CLIENT_ID`, `AUTH_CLIENT_SECRET`,
   `AUTH_ALLOWED_AUDIENCES`, `AUTH_DISABLED`.

3. **Personal Mistral API token in preferences** — "Préférences" panel (account
   menu): `password`-type field, browser-side storage only, never in URL, never
   logged, never returned in a response. Transmitted via `x-mistral-api-token`
   header on generation-triggering calls only. No Mistral key server-side. No
   AI usage in lisage v1: the panel and header mechanics are laid now for
   consistency across the three sites, without a consuming endpoint.

4. **Common interface** — identical header structure across the three sites:
   logo+name, zones (Accueil/Découvrir/Atelier — FR; Home/Explore/Workshop —
   EN), FR/EN language selector, persistent light/dark theme selector, account
   menu. Common components (buttons, cards, badges, toasts, forms, tables;
   monochrome SVG icons, no emoji). Allowed variations: palette, logo, name.
   Accessibility: skip-link, `aria-live`, `document.documentElement.lang`.
   Strict FR/EN bilingual parity with a verification harness. Error pages
   (404, workshop without session) in the same charter.

5. **lisage-specific scope** — crate `lisage-web` depends on `lisage-core` +
   `lisage-liseuse`, never on `lisage-backend`; axum API; React/Vite front.
   `POST /api/workshop-runs`: deterministic, bounded budgets, session/PAT
   protected. Public gallery from archived traces. Diagrams always rendered in
   Rust (front embeds the SVG rendered by the liseuse). Out of scope: manual
   graph editor, textual DSL/LSP, WASM, datacenter scale, online CUDD/Sylvan
   witnesses, multi-user persistence.

6. **Issues WEB-1 to WEB-6** (#330-#335, labels `area/web`, `jalon/WEB`,
   milestone WEB #25) carry the redistributed requirements:
   - WEB-1: `lisage-web` scaffold + bilingual FR/EN harness + theme + header
   - WEB-2: public landing + gallery, no authenticated routes
   - WEB-3: authentication (Authorization Code + PKCE, sessions, PAT, guards)
   - WEB-4: authenticated workshop, protected `POST /api/workshop-runs`,
     `401 session_required` handling, return-to-origin after login
   - WEB-5: preferences panel (Mistral token, theme, language),
     `x-mistral-api-token` mechanics without v1 consumer
   - WEB-6: integration and guardrails (bounded budgets, contract tests,
     ADR-0107 accepted)

7. **Logos** — reference logo files delivered in `docs/lisage-web-v1/logos/`
   (light version, dark version `#0F172A`/`#F1F5F9`, favicon). Deployed as-is
   into the front's static assets at WEB-1; theme selector toggles between light
   and dark versions; favicon deployed unchanged. No retracing or redrawing.

## Consequences

- The three saucisson sites share the same zone/auth/UI contract while remaining
  autonomous repositories.
- No local authentication, no shared secrets, no server-side Mistral keys.
- The WEB milestone issues are the execution vehicle; this ADR is the contract
  of record, to be accepted by the human.

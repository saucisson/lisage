# Mission: saucisson web interface v1 — lisage

## Context

Three saucisson sites must converge: `layered-data.linard.fr`,
`petrichor.linard.fr`, `lisage.linard.fr`. Each site follows the same
model: a public landing page, a public discovery part, a workshop
accessible with an account. Authentication goes only through
`auth.linard.fr` (Zitadel IdP, already deployed and operational); no local
authentication any more. Each user can fill in a personal Mistral API
token in their preferences. The three interfaces adopt the same
navigation structure and the same style, with theme variations per site.
The three repositories stay autonomous: convergence happens through this
common contract, not through a shared package.

This contract is layered on top of the lisage-web framing of 21 November
2026 (ADR-0107, issues WEB-1 to WEB-6, labels `area/web`, `jalon/WEB`):
the framing remains the functional base; this document adds the common
contract on authentication, zones and interface.

Expected conduct:

- Audit the repository's existing state against this contract first;
  check the claims below against the code, and report any gap instead of
  following blindly.
- Create issues WEB-1 to WEB-6 per the framing, folding the requirements
  below into them; move ADR-0107 from draft to accepted after human
  review; work on the `daily` branch per the repository model.
- The site is not online yet: no backward compatibility to preserve,
  implement the target directly.

## 1. The common model: three zones (lisage mapping)

| Zone | Access | lisage contents |
|---|---|---|
| Landing (`/`) | public | Project presentation: what an attributed decision diagram is, who it interests, link to the gallery and the workshop. No authenticated call. |
| Discovery | public | The framed gallery: replayable read-only examples (philosophers n = 3, 4, 5, sliding window), archived traces, SVG rendering by the reader, two editorial registers (novice, BDD notions). |
| Workshop | account required | The framed workshop: write the controller, enable/disable reductions, deterministic `POST /api/workshop-runs` with bounded budgets. |

Cross-cutting rules:

- A visitor who is not logged in on the workshop: explicit message + a
  single button `Se connecter avec auth.linard.fr`; automatic return to
  the page of origin after login.
- Workshop server routes without a session: `401` in
  `application/problem+json`, `reason: "session_required"`. The front
  reacts by redirecting to the login, never by displaying a raw error.
- The landing and the gallery emit no authenticated request and work
  fully without an account.

## 2. Authentication: `auth.linard.fr` only

IdP: Zitadel, issuer `https://auth.linard.fr` (the source of truth is
`/.well-known/openid-configuration`, to be read rather than hard-coded).
Endpoints verified on 21/11/2026:

- authorize: `https://auth.linard.fr/oauth/v2/authorize`
- token: `https://auth.linard.fr/oauth/v2/token`
- JWKS: `https://auth.linard.fr/oauth/v2/keys`
- introspection: `https://auth.linard.fr/oauth/v2/introspect`
- userinfo: `https://auth.linard.fr/oidc/v1/userinfo`
- revocation: `https://auth.linard.fr/oauth/v2/revoke`
- logout: `https://auth.linard.fr/oidc/v1/end_session`

- PKCE `S256` mandatory; scopes `openid profile email`; `ui_locales` fr/en
  accepted; `backchannel_logout` available.
- A confidential OIDC client `lisage` on the Zitadel side; the secret
  lives only server-side.
- Browser flow: Authorization Code + PKCE, driven by axum in `lisage-web`
  (the browser never sees an OIDC token):
  - `GET /auth/login?returnTo=…`: `state` + `nonce` + PKCE S256,
    `ui_locales` per the site language, redirect to authorize; `returnTo`
    validated as an internal path.
  - `GET /auth/callback`: `state`/`nonce` check, code exchange, session
    creation, `returnTo` redirect.
  - `POST /auth/logout`: revocation, `end_session`, cookie destruction.
- Application session: `lisage-session` cookie, `httpOnly`, `Secure`,
  `SameSite=Lax`, bounded lifetime, rotated at login. Server-side storage
  in memory (no multi-user persistence: framing unchanged, the session
  can live in the process memory).
- Machine access: Zitadel PAT (`Authorization: Bearer`) accepted on
  workshop routes in addition to the cookie. Write in `lisage-web` the
  Rust equivalent of the notify brick (local JWT RS256 validation + JWKS
  + introspection); do not reuse the Python.
- `AUTH_DISABLED=1` local-dev-only guard rail: disabled by default, a
  test that fails if it is active outside dev.
- Environment variables: `AUTH_ISSUER`, `AUTH_API_CLIENT_ID`, `AUTH_API_CLIENT_SECRET`, `AUTH_WEB_CLIENT_ID`,
  `AUTH_WEB_CLIENT_SECRET`, `AUTH_ALLOWED_AUDIENCES`, `AUTH_DISABLED` (the
  unified contract of the saucisson repositories).
- No other authentication, no shared secret. The deterministic permalinks
  stay requests: no local application accounts, no app-side roles; the
  account comes from the IdP.

## 3. Personal Mistral token in preferences

- `Préférences` panel (account menu): `Token d'API Mistral` field,
  `password` type, pasteable, masked, clearable; browser-side storage
  only, never in the URL, never logged, never returned in a response.
- Transmission: `x-mistral-api-token` header, only on the calls that
  trigger a generation. No AI usage is planned in lisage v1: lay down the
  panel and the header mechanics now to hold the coherence of the three
  sites, without a consuming endpoint. Without a token: nothing more
  displayed than the panel; the contract will apply when an AI usage
  appears.
- No Mistral key server-side.

## 4. Common interface: navigation, style, theme

- Identical header structure on the three sites, left to right:
  1. Logo + site name (variation per site).
  2. Zones: `Accueil / Découvrir / Atelier` (FR), `Home / Explore / Workshop`
     (EN); `Découvrir` opens the gallery.
  3. FR/EN language selector (framed bilingual from the start).
  4. Persistent light/dark theme selector (classes on `<html>`, default =
     system preference).
  5. Account menu: `Se connecter` if not logged in; username +
     `Préférences` + `Se déconnecter` otherwise.
- Common components: buttons, cards, badges, toasts, forms, tables;
  monochrome SVG pictograms, no emoji.
- Allowed variations: light/dark palette, logo, name. Typography and
  spacing scales identical to the two other sites (layered-data is the
  reference).
- Accessibility: skip-link to the main content, `aria-live` for
  announcements, `document.documentElement.lang` follows the language.
- FR/EN bilingual: strict parity of the message catalogues, with a
  required parity-check harness; two editorial registers (novice, BDD
  notions) per the framing, without confusion with the FR/EN mechanics.
- Error pages (404, workshop without session) in the same charter.
- Framing reminders, unchanged: diagram rendering always in Rust (the
  front embeds the SVG rendered by the reader, it does not redo the
  layout), static gallery rebuilt byte-identical, measured non-switchable
  R-share, no ghost switches (every displayed switch has a measured
  effect), scrubber = trace replay.

## 5. Work specific to lisage

- Crate `lisage-web`: depends on `lisage-core` + `lisage-liseuse`, never
  on `lisage-backend`; axum for the API; React/Vite front (port of the
  validated prototype).
- `POST /api/workshop-runs`: deterministic, response = report + JSONL v3
  trace + SVG (the permalink is the request), bounded budgets; route
  protected by session/PAT (`401 session_required` otherwise).
- Public gallery: pages generated from the archived traces, no
  authentication.
- Authenticated workshop: the complete workshop behind the login.
- Assumed out of scope (framing unchanged): no manual graph editor, no
  text DSL/LSP, no WASM, no datacenter scale or online CUDD/Sylvan
  witness, no multi-user persistence.
- ADR-0107: fold the auth/zone sections of this contract in, status to
  move from draft to accepted.

## 6. Issues to create in this repository

Create the framing's WEB-1 to WEB-6 issues (labels `area/web`,
`jalon/WEB`), distributing the requirements of this contract as follows:

1. **WEB-1: `lisage-web` scaffolding**: axum crate + React/Vite front,
   FR/EN bilingual with a parity harness, light/dark theme, compliant
   header (incl. account menu and language selector).
2. **WEB-2: landing and public gallery**: welcome landing, gallery from
   the archived traces, no authenticated route.
3. **WEB-3: authentication**: Authorization Code + PKCE flow,
   `lisage-session` sessions, PAT, Rust equivalent of the notify brick,
   `AUTH_DISABLED` guard rails.
4. **WEB-4: authenticated workshop**: complete workshop, protected
   `POST /api/workshop-runs`, `401 session_required` handling on the
   front, return to the page of origin after login.
5. **WEB-5: preferences**: Preferences panel (Mistral token, theme,
   language), `x-mistral-api-token` mechanics laid down without a
   consumer in v1.
6. **WEB-6: integration and guard rails**: bounded budgets, contract tests
   (public navigation, protected workshop, complete login/logout),
   ADR-0107 accepted.

## 7. Definition of Done

- Gallery and landing fully functional without a session; workshop
  without a session: login redirect then return to the page of origin.
- Login / callback / logout: complete flow successful against
  `auth.linard.fr`; PAT accepted on the workshop.
- Byte-identical gallery rebuild; deterministic
  `POST /api/workshop-runs`; bounded budgets respected.
- Green FR/EN i18n harness; no static secret left; guard rails tested
  (`AUTH_DISABLED` disabled by default).
- ADR-0107 accepted; labels `area/web`, `jalon/WEB` created; no ghost
  switch.

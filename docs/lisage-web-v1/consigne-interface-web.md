# Mission : interface web saucisson v1 — lisage

## Contexte

Trois sites saucisson doivent converger : `layered-data.linard.fr`, `petrichor.linard.fr`, `lisage.linard.fr`. Chaque site suit le même modèle : une landing page publique, une partie découverte publique, un atelier accessible avec compte. L'authentification passe uniquement par `auth.linard.fr` (IdP Zitadel, déjà déployé et opérationnel) ; plus aucune authentification locale. Chaque utilisateur peut renseigner dans ses préférences un token d'API Mistral personnel. Les trois interfaces adoptent la même structure de navigation et le même style, avec des variations de thème par site. Les trois dépôts restent autonomes : la convergence se fait par ce contrat commun, pas par un package partagé.

Ce contrat se superpose au cadrage lisage-web du 21 novembre 2026 (ADR-0107, issues WEB-1 à WEB-6, labels `area/web`, `jalon/WEB`) : le cadrage reste la base fonctionnelle ; ce document ajoute le contrat commun d'authentification, de zones et d'interface.

Conduite attendue :

- Audite d'abord l'existant du dépôt au regard de ce contrat ; vérifie les affirmations ci-dessous contre le code, et signale tout écart au lieu de le suivre aveuglément.
- Crée les issues WEB-1 à WEB-6 selon le cadrage, en y intégrant les exigences ci-dessous ; fais passer l'ADR-0107 de draft à acceptée après revue humaine ; travaille sur la branche `daily` selon le modèle du dépôt.
- Le site n'est pas encore en ligne : aucune compatibilité à préserver, implémente directement la cible.

## 1. Le modèle commun : trois zones (mapping lisage)

| Zone | Accès | Contenu lisage |
|---|---|---|
| Atterrissage (`/`) | publique | Présentation du projet : ce qu'est un diagramme de décision attribué, qui ça intéresse, lien vers la galerie et l'atelier. Aucun appel authentifié. |
| Découverte | publique | La galerie cadrée : exemples rejouables en lecture seule (philosophes n = 3, 4, 5, fenêtre glissante), traces archivées, rendu SVG par la liseuse, deux registres éditoriaux (novice, notions de BDD). |
| Atelier | compte requis | Le workshop cadré : écrire le contrôleur, activer/désactiver les réductions, `POST /api/workshop-runs` déterministe avec budgets bornés. |

Règles transverses :

- Visiteur non connecté sur l'atelier : message explicite + bouton unique « Se connecter avec auth.linard.fr » ; retour automatique à la page d'origine après connexion.
- Routes serveur de l'atelier sans session : `401` en `application/problem+json`, `reason: "session_required"`. Le front réagit en redirigeant vers la connexion, jamais en affichant une erreur brute.
- La landing et la galerie n'émettent aucune requête authentifiée et fonctionnent intégralement sans compte.

## 2. Authentification : `auth.linard.fr` uniquement

IdP : Zitadel, issuer `https://auth.linard.fr` (la source de vérité est `/.well-known/openid-configuration`, à lire plutôt que hard-coder). Endpoints vérifiés le 21/11/2026 :

- authorize : `https://auth.linard.fr/oauth/v2/authorize`
- token : `https://auth.linard.fr/oauth/v2/token`
- JWKS : `https://auth.linard.fr/oauth/v2/keys`
- introspection : `https://auth.linard.fr/oauth/v2/introspect`
- userinfo : `https://auth.linard.fr/oidc/v1/userinfo`
- révocation : `https://auth.linard.fr/oauth/v2/revoke`
- déconnexion : `https://auth.linard.fr/oidc/v1/end_session`

- PKCE `S256` obligatoire ; scopes `openid profile email` ; `ui_locales` fr/en acceptés ; `backchannel_logout` disponible.
- Un client OIDC confidentiel `lisage` côté Zitadel ; le secret ne vit que côté serveur.
- Flux navigateur : Authorization Code + PKCE, piloté par axum dans `lisage-web` (le navigateur ne voit jamais de token OIDC) :
  - `GET /auth/login?returnTo=…` : `state` + `nonce` + PKCE S256, `ui_locales` selon la langue du site, redirection vers authorize ; `returnTo` validé comme chemin interne.
  - `GET /auth/callback` : vérification `state`/`nonce`, échange du code, création de session, redirection `returnTo`.
  - `POST /auth/logout` : révocation, `end_session`, destruction du cookie.
- Session applicative : cookie `lisage-session`, `httpOnly`, `Secure`, `SameSite=Lax`, durée de vie bornée, rotation à la connexion. Stockage serveur en mémoire (pas de persistance multi-utilisateur : cadrage inchangé, la session peut vivre en mémoire du process).
- Accès machines : PAT Zitadel (`Authorization: Bearer`) accepté sur les routes atelier en plus du cookie. Écrire dans `lisage-web` l'équivalent Rust de la brique notify (validation locale JWT RS256 + JWKS + introspection) ; ne pas réutiliser le Python.
- `AUTH_DISABLED=1` garde-fou dev local uniquement : désactivé par défaut, test qui échoue s'il est actif hors dev.
- Variables d'environnement : `AUTH_ISSUER`, `AUTH_CLIENT_ID`, `AUTH_CLIENT_SECRET`, `AUTH_ALLOWED_AUDIENCES`, `AUTH_DISABLED` (contrat unifié des dépôts saucisson).
- Aucune autre authentification, aucun secret partagé. Les permaliens déterministes restent des requêtes : pas de comptes applicatifs locaux, pas de rôles côté app ; le compte vient de l'IdP.

## 3. Token Mistral personnel dans les préférences

- Panneau « Préférences » (menu compte) : champ « Token d'API Mistral », type `password`, collable, masqué, effaçable ; stockage navigateur uniquement, jamais dans l'URL, jamais logué, jamais renvoyé dans une réponse.
- Transmission : header `x-mistral-api-token`, uniquement sur les appels qui déclenchent une génération. Aucun usage IA n'est prévu dans la v1 lisage : poser le panneau et la mécanique du header dès maintenant pour tenir la cohérence des trois sites, sans endpoint consommateur. Sans token : rien d'affiché de plus que le panneau ; le contrat s'appliquera quand un usage IA apparaitra.
- Aucune clé Mistral côté serveur.

## 4. Interface commune : navigation, style, thème

- Header identique en structure sur les trois sites, de gauche à droite :
  1. Logo + nom du site (variation par site).
  2. Zones : « Accueil / Découvrir / Atelier » (FR), « Home / Explore / Workshop » (EN) ; « Découvrir » ouvre la galerie.
  3. Sélecteur de langue FR/EN (cadré bilingue dès le départ).
  4. Sélecteur de thème clair/sombre persistant (classes sur `<html>`, défaut = préférence système).
  5. Menu compte : « Se connecter » si non connecté ; nom d'utilisateur + « Préférences » + « Se déconnecter » sinon.
- Composants communs : boutons, cartes, badges, toasts, formulaires, tableaux ; pictogrammes SVG monochromes, pas d'emoji.
- Variations autorisées : palette light/dark, logo, nom. Typographie et échelles d'espacement identiques aux deux autres sites (layered-data sert de référence).
- Accessibilité : skip-link vers le contenu principal, `aria-live` pour les annonces, `document.documentElement.lang` suit la langue.
- Bilingue FR/EN : parité stricte des catalogues de messages, avec harnais de vérification de parité exigé ; deux registres éditoriaux (novice, notions de BDD) selon le cadrage, sans confusion avec la mécanique FR/EN.
- Pages d'erreur (404, atelier sans session) dans la même charte.
- Rappels du cadrage inchangés : rendu des diagrammes toujours en Rust (le front embarque le SVG rendu par la liseuse, il ne refait pas le layout), galerie statique rebuild byte-identical, R-share mesuré non switchable, pas d'interrupteurs fantômes (tout interrupteur affiché a un effet mesuré), scrubber = rejeu de trace.

## 5. Travail propre à lisage

- Crate `lisage-web` : dépend de `lisage-core` + `lisage-liseuse`, jamais de `lisage-backend` ; axum pour l'API ; front React/Vite (portage du prototype validé).
- `POST /api/workshop-runs` : déterministe, réponse = rapport + trace JSONL v3 + SVG (le permalien est la requête), budgets bornés ; route protégée par session/PAT (`401 session_required` sinon).
- Galerie publique : pages générées à partir des traces archivées, aucune authentification.
- Atelier authentifié : le workshop complet derrière la connexion.
- Hors périmètre assumé (cadrage inchangé) : pas d'éditeur de graphe manuel, pas de DSL textuel/LSP, pas de WASM, pas d'échelle datacenter ni de témoin CUDD/Sylvan en ligne, pas de persistance multi-utilisateur.
- ADR-0107 : intégrer les sections auth/zones de ce contrat, statut à faire passer de draft à accepté.

## 6. Issues à créer dans ce dépôt

Créer les issues WEB-1 à WEB-6 du cadrage (labels `area/web`, `jalon/WEB`), en répartissant les exigences de ce contrat ainsi :

1. **WEB-1 : charpente `lisage-web`** : crate axum + front React/Vite, bilingue FR/EN avec harnais de parité, thème light/dark, header conforme (incl. menu compte et sélecteur de langue).
2. **WEB-2 : landing et galerie publique** : landing d'accueil, galerie depuis les traces archivées, aucune route authentifiée.
3. **WEB-3 : authentification** : flux Authorization Code + PKCE, sessions `lisage-session`, PAT, équivalent Rust de la brique notify, garde-fous `AUTH_DISABLED`.
4. **WEB-4 : atelier authentifié** : workshop complet, `POST /api/workshop-runs` protégé, gestion `401 session_required` côté front, retour à la page d'origine après connexion.
5. **WEB-5 : préférences** : panneau Préférences (token Mistral, thème, langue), mécanique `x-mistral-api-token` posée sans consommateur en v1.
6. **WEB-6 : intégration et garde-fous** : budgets bornés, tests du contrat (navigation publique, atelier protégé, login/logout complet), ADR-0107 acceptée.

## 7. Definition of Done

- Galerie et landing intégralement fonctionnelles sans session ; atelier sans session : redirection login puis retour à la page d'origine.
- Login / callback / logout : parcours complet réussi contre `auth.linard.fr` ; PAT accepté sur le workshop.
- Rebuild byte-identical de la galerie ; `POST /api/workshop-runs` déterministe ; budgets bornés respectés.
- Harnais i18n FR/EN vert ; plus aucun secret statique ; garde-fous testés (`AUTH_DISABLED` désactivé par défaut).
- ADR-0107 acceptée ; labels `area/web`, `jalon/WEB` créés ; pas d'interrupteur fantôme.

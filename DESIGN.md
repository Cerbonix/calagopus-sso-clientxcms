# Design

Everything below was read in the panel source at tag `release-1.1.4`, the version this
extension targets. Line references point at that tag, not at the repository head,
which runs ahead of every released image.

The guiding rule: reuse what the panel already does. The ticket flow introduces no new
cryptographic primitive, no new storage mechanism and no new session concept. Every
piece below already backs the panel's own OAuth login.

## The flow

Two routes, both mounted through `add_auth_api_router` (`shared/src/extensions/mod.rs:73-84`).
That mount point and `add_global_router` are the only ones the parent router does not
put behind session authentication, which is exactly what this needs: the first route is
called machine to machine, the second is opened by a browser that is not signed in yet.

| Step | Route | What happens |
|---|---|---|
| 1 | `POST /api/auth/<pkg>/tickets` | The billing platform sends the shared secret in the JSON body and the customer it wants a ticket for. The secret is compared in constant time. A short lived token is written to the cache against the target user uuid. The ticket URL is returned. |
| 2 | `GET /api/auth/<pkg>/tickets/{token}` | The browser opens it. The token is read then immediately invalidated, a session is created, the cookie is set, and the browser is redirected to the server. |

## What it reuses

| Need | Panel primitive | Source |
|---|---|---|
| Store the shared secret, encrypted at rest | `SettingsSerializer::write_raw_encrypted_setting` / `read_raw_encrypted_setting` | `shared/src/extensions/settings.rs:50-60`, `:164-176` |
| Encryption itself | `Database::encrypt_base64` / `decrypt_base64`, keyed by the instance encryption key | `shared/src/database.rs:164-168`, `:254-258` |
| Hold the pending ticket with a TTL | `state.cache` set / get / invalidate, the very mechanism the panel uses for the OAuth CSRF state | `shared/src/cache.rs:488-522`, `:449-466`, `:595-602` |
| Create the session | `UserSession::create` with user uuid, ip and user agent | `shared/src/models/user_session.rs:274-331` |
| Set the cookie | `UserSession::get_cookie` plus `cookies.add`, via the `tower_cookies` extractor the core installs globally | `shared/src/models/user_session.rs:221-239` |
| Reference implementation of steps 4 and 5 | The panel's own OAuth callback does exactly this, then redirects | `backend/src/routes/api/auth/oauth/_oauth_provider_.rs:359-373`, `:415-418` |

The session value returned by `create` is `"{key_id}:{hash}"`, and only a bcrypt hash of
it is stored (`user_session.rs:309-326`). Nothing readable is persisted.

## Constraints found the hard way

**The package name cannot contain an underscore, and cannot contain a dash either.**
The identifier segment is validated to lowercase, digits and `-` only, 4 to 30 characters
(`shared/src/extensions/distr.rs:520-549`). But the generated extension list injects
`package_name.replace('.', "_")` straight into Rust source as a crate path
(`distr.rs:973`). A dash survives that replacement and produces an invalid identifier,
which fails the build of the whole panel, not just this extension. So: lowercase and
digits only. Hence `net.cerbonix.ssotickets`.

**A frontend is mandatory even for an API-only extension.** The archive validator requires
`frontend/package.json` and a `frontend/src/index.ts` containing `export default`
(`distr.rs:591-604`). Ours renders nothing and exists to satisfy the format.

**Constant-time comparison is not available from `shared`.** Neither `subtle` nor
`constant_time_eq` is a direct workspace dependency, so this crate declares its own.

**Sessions created here last as long as any other login.** `get_cookie` derives expiry
from the instance-wide `session_duration_seconds` setting (`user_session.rs:232-237`).
There is no per-session override, so a ticket-issued session cannot be made shorter
without stepping outside the panel's own mechanism.

## Deployment

`panel-rs extensions add <file>.c7s.zip` unpacks and validates, then
`panel-rs extensions apply` builds the frontend and recompiles the panel binary with the
extension statically linked (`backend/src/commands/extensions/apply.rs:264-305`).
Over HTTP the equivalent is `POST /api/admin/extensions/manage/rebuild`, permission
`extensions.manage`, which refuses with `501` outside the heavy container
(`.../manage/rebuild/mod.rs:39-44`).

## Settings have no generic admin API

The core exposes settings routes for itself (`backend/src/routes/api/admin/settings.rs`)
but nothing generic for extensions: `/api/admin/extensions/manage/*` manages the package,
not its configuration. This extension therefore exposes its own admin route through
`add_admin_api_router`, declaring its permission via `initialize_permissions`
(`shared/src/extensions/mod.rs:192-200`).

## Still unverified

- Where and when `database/extension-migrations/` is actually executed. Neither `add` nor
  `apply` calls a migration routine; the boot path was not located.
- Whether the decryption cache is enabled by default (`database.rs:186`).
- Whether `User::by_external_id` exists, which would resolve the target account directly.
- The exact signatures of `Cache::lock` and `Cache::ratelimit`, needed to close the race
  between reading and invalidating a ticket under concurrent requests.

# ClientXCMS SSO

Calagopus panel extension. It lets a billing platform sign one of its customers into
the panel without ever knowing or setting that customer's panel password.

ClientXCMS calls this extension server to server with a shared secret, gets back a
single-use login ticket, and redirects the customer's browser onto it.

> **Work in progress.** Nothing is implemented yet: this repository currently holds
> the extension skeleton only. Do not install it.

## Why an extension and not OAuth2

Calagopus can already consume a generic OAuth2 provider, so the billing platform
could have become an identity provider instead. That path was dropped: it means
writing an authorization server, and a mistake there is not a rendering bug, it is
an account takeover. This extension reuses the panel's own session mechanism and
introduces no new cryptographic primitive.

## Requirements

| | |
|---|---|
| Panel | `>=1.1.4` |
| Image | An **official `heavy` image** (`ghcr.io/calagopus/panel:<version>-heavy`) |

Extensions are Rust crates compiled into the panel binary, not plugins dropped into
a folder. The standard panel image carries no toolchain and will refuse to manage
extensions at all:

```
extension management is only available in the official heavy container
```

Two ways to run it, pick either:

- run a `heavy` image and rebuild from the panel admin, or
- build your own image in CI and deploy only the result, keeping a compiler out of
  production.

Either way the extension has to be rebuilt whenever the panel is upgraded. The panel
ships weekly.

## Security requirements

These are requirements, not suggestions. They exist because the equivalent
Pterodactyl integration gets several of them wrong.

| Rule | Why |
|---|---|
| The shared secret never travels in a query string | Query strings land in access logs, in proxy logs, and leak through `Referer` |
| The shared secret is compared in constant time | Avoids leaking it one byte at a time |
| The shared secret is rotatable without redeploying | An unrotatable credential is a permanent one |
| Tickets are single use and invalidated on first use | A replayed ticket is a second, unauthorized session |
| Tickets are short lived | Bounds the window if one leaks |
| Tickets are bound to the user they were issued for | Otherwise a ticket becomes a key to any account |
| A wrong secret and a missing secret fail alike | Never let a caller learn which half was wrong |

## Layout

```
Metadata.toml        package name, display name, supported panel range
backend/             Rust crate, implements the shared::extensions::Extension trait
```

Trait reference: https://cratedocs.calagopus.com/shared/extensions/trait.Extension

## License

Copyright (c) 2026 Cerbonix. Licensing terms are not settled yet.

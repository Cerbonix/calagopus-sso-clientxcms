# ClientXCMS SSO

Extension pour le panel Calagopus. Elle permet à une plateforme de facturation de connecter un de ses clients sur le panel, sans jamais connaître ni définir le mot de passe de ce client.

ClientXCMS appelle cette extension de serveur à serveur avec un secret partagé, reçoit un ticket de connexion à usage unique, et redirige le navigateur du client dessus.

> **En cours de développement, mais le socle est vérifié.** L'extension compile, s'installe et se charge réellement sur un panel `1.1.4` (voir la trace de démarrage `clientxcms sso extension loaded`).
>
> En revanche elle n'émet encore **aucun ticket** : la logique décrite ci-dessous n'est pas implémentée. L'installer aujourd'hui n'apporte donc rien, et vous ferait basculer sur l'image `heavy` sans contrepartie.

## Pourquoi une extension et pas OAuth2

Calagopus sait déjà consommer un fournisseur OAuth2 générique : la plateforme de facturation aurait donc pu devenir fournisseur d'identité. Cette voie a été écartée, parce qu'elle revient à écrire un serveur d'autorisation, et qu'une erreur à cet endroit ne se paie pas en défaut d'affichage mais en prise de contrôle de compte. Cette extension réutilise le mécanisme de session du panel lui-même et n'introduit aucune primitive cryptographique nouvelle.

## Prérequis

| | |
|---|---|
| Panel | `>=1.1.4` |
| Image | Une image **`heavy` officielle** (`ghcr.io/calagopus/panel:<version>-heavy`) |

Les extensions sont des crates Rust compilés dans le binaire du panel, pas des greffons déposés dans un dossier. L'image standard ne contient aucune chaîne de compilation et refuse purement et simplement de gérer des extensions :

```
extension management is only available in the official heavy container
```

### L'image heavy exige quatre volumes

Sans eux, elle ne démarre pas et boucle sur `/app/binaries is missing or is not a directory` :

```yaml
volumes:
  - ./build/binaries:/app/binaries
  - ./build/translations:/app/translations
  - ./build/extensions:/app/extensions
  - ./build/extension-migrations:/app/repo/database/extension-migrations
```

Conservez la même valeur d'`APP_ENCRYPTION_KEY` qu'auparavant : elle chiffre les secrets déjà présents en base.

### Installation

Deux voies, mais elles ne sont pas équivalentes :

| Voie | Résultat |
|---|---|
| `PUT /api/admin/extensions/manage/add?accept_license=true`, archive en corps binaire brut, puis `POST /api/admin/extensions/manage/rebuild` | **Correct.** L'archive arrive dans `/app/extensions`, que le superviseur lit |
| `panel-rs extensions add` en ligne de commande | **Insuffisant en mode heavy.** L'archive est écrite dans `/app/repo/backend-extensions`, que le superviseur ignore. La reconstruction répond alors `these inputs are already built, nothing to do` et le binaire produit ne contient pas l'extension |

Comptez une dizaine de minutes pour la première compilation, quelques dizaines de secondes ensuite tant que le cache de compilation reste chaud. L'extension doit être reconstruite à chaque montée de version du panel, qui publie chaque semaine.

## Exigences de sécurité

Ce sont des exigences, pas des suggestions. Elles existent parce que l'intégration Pterodactyl équivalente se trompe sur plusieurs d'entre elles.

| Règle | Pourquoi |
|---|---|
| Le secret partagé ne transite jamais en paramètre de requête | Les chaînes de requête finissent dans les journaux d'accès, dans ceux des mandataires, et fuient par l'en-tête `Referer` |
| Le secret est comparé en temps constant | Évite de le livrer octet par octet |
| Le secret est rotatif sans redéploiement | Un identifiant qu'on ne peut pas changer est un identifiant définitif |
| Le ticket est à usage unique, invalidé dès la première utilisation | Un ticket rejoué est une seconde session, non autorisée |
| Le ticket a une durée de vie courte | Borne la fenêtre d'exploitation en cas de fuite |
| Le ticket est lié à l'utilisateur pour lequel il a été émis | Sinon il devient une clé vers n'importe quel compte |
| Un secret absent échoue comme un secret erroné | Ne jamais laisser l'appelant apprendre laquelle des deux moitiés était fausse |

## Structure

```
Metadata.toml        nom de paquet, nom affiché, plage de versions supportées
backend/             crate Rust, implémente le trait shared::extensions::Extension
frontend/            point d'entrée exigé par le format, ne rend rien ici
```

Le nom de paquet ne peut contenir **ni tiret bas, ni tiret**. Le tiret bas est rejeté à la validation, et le tiret passe la validation mais casse la compilation : le nom est injecté tel quel comme chemin de crate Rust dans la liste d'extensions générée.

Référence du trait : https://cratedocs.calagopus.com/shared/extensions/trait.Extension

La conception détaillée, avec ses sources, est dans [DESIGN.md](./DESIGN.md).

## Licence

Copyright (c) 2026 Cerbonix. Les conditions de licence ne sont pas encore arrêtées.

---

# ClientXCMS SSO (English)

Calagopus panel extension. It lets a billing platform sign one of its customers into the panel without ever knowing or setting that customer's panel password.

ClientXCMS calls this extension server to server with a shared secret, gets back a single-use login ticket, and redirects the customer's browser onto it.

> **Work in progress, but the groundwork is proven.** The extension compiles, installs and actually loads on a `1.1.4` panel (see the `clientxcms sso extension loaded` startup line).
>
> It does **not issue any ticket** yet: the logic described below is not implemented. Installing it today buys you nothing, and moves you onto the `heavy` image for no return.

## Why an extension and not OAuth2

Calagopus can already consume a generic OAuth2 provider, so the billing platform could have become an identity provider instead. That path was dropped: it means writing an authorization server, and a mistake there is not a rendering bug, it is an account takeover. This extension reuses the panel's own session mechanism and introduces no new cryptographic primitive.

## Requirements

| | |
|---|---|
| Panel | `>=1.1.4` |
| Image | An **official `heavy` image** (`ghcr.io/calagopus/panel:<version>-heavy`) |

Extensions are Rust crates compiled into the panel binary, not plugins dropped into a folder. The standard image carries no toolchain and flatly refuses to manage extensions:

```
extension management is only available in the official heavy container
```

### The heavy image needs four volumes

Without them it will not boot, looping on `/app/binaries is missing or is not a directory`:

```yaml
volumes:
  - ./build/binaries:/app/binaries
  - ./build/translations:/app/translations
  - ./build/extensions:/app/extensions
  - ./build/extension-migrations:/app/repo/database/extension-migrations
```

Keep the same `APP_ENCRYPTION_KEY` as before: it decrypts the secrets already stored in your database.

### Installing

Two routes, and they are not equivalent:

| Route | Outcome |
|---|---|
| `PUT /api/admin/extensions/manage/add?accept_license=true` with the archive as the raw body, then `POST /api/admin/extensions/manage/rebuild` | **Correct.** The archive lands in `/app/extensions`, which the supervisor reads |
| `panel-rs extensions add` on the command line | **Not enough on the heavy image.** The archive is written to `/app/repo/backend-extensions`, which the supervisor ignores. The rebuild then answers `these inputs are already built, nothing to do` and the resulting binary does not carry the extension |

Expect around ten minutes for the first build, then a matter of seconds while the compilation cache stays warm. The extension must be rebuilt on every panel upgrade, and the panel ships weekly.

## Security requirements

These are requirements, not suggestions. They exist because the equivalent Pterodactyl integration gets several of them wrong.

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
frontend/            entrypoint the archive format requires, renders nothing here
```

The package name can contain **neither an underscore nor a dash**. The underscore is rejected by validation, and the dash passes validation but breaks the build: the name is injected verbatim as a Rust crate path into the generated extension list.

Trait reference: https://cratedocs.calagopus.com/shared/extensions/trait.Extension

The detailed design, with its sources, lives in [DESIGN.md](./DESIGN.md).

## License

Copyright (c) 2026 Cerbonix. Licensing terms are not settled yet.

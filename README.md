# ClientXCMS SSO

Extension pour le panel Calagopus. Elle permet à une plateforme de facturation de connecter un de ses clients sur le panel, sans jamais connaître ni définir le mot de passe de ce client.

ClientXCMS appelle cette extension de serveur à serveur avec un secret partagé, reçoit un ticket de connexion à usage unique, et redirige le navigateur du client dessus.

> **Fonctionnelle, vérifiée de bout en bout sur un panel `1.1.4`.** Un ticket est émis, consommé une seule fois, ouvre une session valide, et un rejeu répond `401`. La rotation du secret a également été vérifiée en service.
>
> Les conditions de licence ne sont pas encore arrêtées : voir la section Licence avant tout usage commercial.

## Pourquoi une extension et pas OAuth2

Calagopus sait déjà consommer un fournisseur OAuth2 générique : la plateforme de facturation aurait donc pu devenir fournisseur d'identité. Cette voie a été écartée, parce qu'elle revient à écrire un serveur d'autorisation, et qu'une erreur à cet endroit ne se paie pas en défaut d'affichage mais en prise de contrôle de compte. Cette extension réutilise le mécanisme de session du panel lui-même et n'introduit aucune primitive cryptographique nouvelle.

## Prérequis

| | |
|---|---|
| Panel | `>=1.1.4` |
| Image | Une image **`heavy` officielle** (`ghcr.io/calagopus/panel:<version>-heavy`) |
| Outil local | `zip`, pour empaqueter les sources. Aucune chaîne de compilation Rust n'est requise chez vous |

Les extensions sont des crates Rust compilés dans le binaire du panel, pas des greffons déposés dans un dossier. L'image standard ne contient aucune chaîne de compilation et refuse purement et simplement de gérer des extensions :

```
extension management is only available in the official heavy container
```

### L'image heavy exige quatre volumes

Sans eux, elle ne démarre pas et boucle sur `/app/binaries is missing or is not a directory` :

À ajouter au service du panel dans votre `docker-compose.yml`, puis recréer le conteneur :

```yaml
volumes:
  - ./build/binaries:/app/binaries
  - ./build/translations:/app/translations
  - ./build/extensions:/app/extensions
  - ./build/extension-migrations:/app/repo/database/extension-migrations
```

Conservez la même valeur d'`APP_ENCRYPTION_KEY` qu'auparavant : elle chiffre les secrets déjà présents en base.

### Construire l'archive

Une extension s'installe sous forme d'archive `zip`. Vous n'avez **rien à compiler vous-même** : le panel compile les sources à l'installation, c'est tout l'objet de l'image `heavy`. Il suffit donc d'empaqueter les sources telles quelles.

```sh
git clone https://github.com/Cerbonix/calagopus-sso-clientxcms.git
cd calagopus-sso
zip -r ../net_cerbonix_ssotickets.c7s.zip Metadata.toml backend frontend \
  -x '*/target/*' '*/node_modules/*'
```

L'archive doit contenir `Metadata.toml`, `backend/` et `frontend/` **à sa racine**, sans dossier intermédiaire. C'est la structure du modèle officiel livré avec le panel, `.extension-templates/com_calagopus_template1.c7s.zip`.

Le nom du fichier est libre : le panel lit le `package_name` du `Metadata.toml` et ignore le nom de l'archive. La convention `<package_name, points remplacés par des tirets bas>.c7s.zip` reste la plus lisible.

### Installer

Par l'interface du panel, dans **Admin > Extensions** (`/admin/extensions`) : le bouton d'ajout accepte un fichier `.zip`, puis la reconstruction se lance et affiche son avancement. C'est la voie recommandée, et la seule qui ne demande aucun outillage.

En ligne de commande, si vous automatisez :

```sh
curl -X PUT "https://panel.example.net/api/admin/extensions/manage/add?accept_license=true" \
  -H "Authorization: Bearer <clé API admin>" \
  --data-binary @net_cerbonix_ssotickets.c7s.zip

curl -X POST "https://panel.example.net/api/admin/extensions/manage/rebuild" \
  -H "Authorization: Bearer <clé API admin>"
```

**N'utilisez pas `panel-rs extensions add`.** En mode heavy, l'archive est écrite dans `/app/repo/backend-extensions`, que le superviseur ignore. La reconstruction répond alors `these inputs are already built, nothing to do` et le binaire produit ne contient pas l'extension.

Comptez une dizaine de minutes pour la première compilation, quelques dizaines de secondes ensuite tant que le cache de compilation reste chaud. L'extension doit être reconstruite à chaque montée de version du panel, qui publie chaque semaine.

### Configurer le secret partagé

**Depuis ClientXCMS, en une commande, sans rien saisir ici :**

```sh
php artisan calagopus:sso
```

Elle tire un secret au hasard, le pose sur le panel par la route ci-dessous, puis le conserve chiffré de son côté. C'est la voie normale, et elle garantit que les deux moitiés correspondent.

La clé API que ClientXCMS utilise doit porter la permission **`ssotickets.manage`**, que cette extension ajoute au panel une fois installée.

Si vous pilotez le panel seul, la route existe :

```sh
curl -X PUT "https://panel.example.net/api/admin/ssotickets/secret" \
  -H "Authorization: Bearer <clé portant ssotickets.manage>" \
  -H "Content-Type: application/json" \
  -d '{"secret":"<au moins 32 caractères>"}'
```

Le secret est stocké chiffré dans les réglages de l'extension. La rotation consiste à rejouer `php artisan calagopus:sso` : aucune reconstruction, aucun redémarrage.

### Vérifier que ça marche

Dans ClientXCMS, ouvrez la fiche du serveur et lancez **Tester la connexion** : il annonce explicitement si l'authentification unique est configurée. Puis, depuis un compte client disposant d'un service actif, cliquez sur « Ouvrir le panel » : vous devez arriver connecté, sans page de connexion.

Au démarrage du panel, la trace `clientxcms sso extension loaded` confirme que l'extension est bien dans le binaire en cours d'exécution.

### Si ça ne marche pas

| Symptôme | Cause probable |
|---|---|
| `extension management is only available in the official heavy container` | Panel en image standard, voir Prérequis |
| Le panel boucle sur `/app/binaries is missing or is not a directory` | Les quatre volumes ne sont pas montés |
| `these inputs are already built, nothing to do` et extension absente | Installée via `panel-rs extensions add`, voir ci-dessus |
| La reconstruction échoue | Les journaux de compilation sont exposés par `GET /api/admin/extensions/manage/logs`, et affichés dans **Admin > Extensions** |
| Le client atterrit sur la page de connexion du panel | Secret absent ou désaccordé : rejouez `php artisan calagopus:sso` |
| ClientXCMS signale une permission manquante | La clé API n'a pas `ssotickets.manage` |

Pour connaître votre version de panel : elle est affichée dans l'administration du panel, et lisible par `GET /api/admin/system/overview`.

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

> **Working, verified end to end against a `1.1.4` panel.** A ticket is issued, consumed exactly once, opens a valid session, and a replay answers `401`. Secret rotation was verified in service too.
>
> Licensing terms are not settled yet: read the License section before any commercial use.

## Why an extension and not OAuth2

Calagopus can already consume a generic OAuth2 provider, so the billing platform could have become an identity provider instead. That path was dropped: it means writing an authorization server, and a mistake there is not a rendering bug, it is an account takeover. This extension reuses the panel's own session mechanism and introduces no new cryptographic primitive.

## Requirements

| | |
|---|---|
| Panel | `>=1.1.4` |
| Image | An **official `heavy` image** (`ghcr.io/calagopus/panel:<version>-heavy`) |
| Local tool | `zip`, to package the sources. No Rust toolchain is needed on your side |

Extensions are Rust crates compiled into the panel binary, not plugins dropped into a folder. The standard image carries no toolchain and flatly refuses to manage extensions:

```
extension management is only available in the official heavy container
```

### The heavy image needs four volumes

Without them it will not boot, looping on `/app/binaries is missing or is not a directory`:

Add them to the panel service in your `docker-compose.yml`, then recreate the container:

```yaml
volumes:
  - ./build/binaries:/app/binaries
  - ./build/translations:/app/translations
  - ./build/extensions:/app/extensions
  - ./build/extension-migrations:/app/repo/database/extension-migrations
```

Keep the same `APP_ENCRYPTION_KEY` as before: it decrypts the secrets already stored in your database.

### Building the archive

An extension installs as a `zip` archive. You compile **nothing yourself**: the panel builds the sources on install, which is the whole point of the `heavy` image. So you only package the sources as they are.

```sh
git clone https://github.com/Cerbonix/calagopus-sso-clientxcms.git
cd calagopus-sso
zip -r ../net_cerbonix_ssotickets.c7s.zip Metadata.toml backend frontend \
  -x '*/target/*' '*/node_modules/*'
```

The archive must carry `Metadata.toml`, `backend/` and `frontend/` **at its root**, with no intermediate folder. That is the layout of the official template shipped with the panel, `.extension-templates/com_calagopus_template1.c7s.zip`.

The file name is free: the panel reads `package_name` from `Metadata.toml` and ignores the archive name. The `<package_name, dots replaced with underscores>.c7s.zip` convention just stays the most readable.

### Installing

Through the panel interface, under **Admin > Extensions** (`/admin/extensions`): the add button takes a `.zip` file, then the rebuild starts and reports its progress. This is the recommended route, and the only one needing no tooling at all.

On the command line, if you automate:

```sh
curl -X PUT "https://panel.example.net/api/admin/extensions/manage/add?accept_license=true" \
  -H "Authorization: Bearer <admin API key>" \
  --data-binary @net_cerbonix_ssotickets.c7s.zip

curl -X POST "https://panel.example.net/api/admin/extensions/manage/rebuild" \
  -H "Authorization: Bearer <admin API key>"
```

**Do not use `panel-rs extensions add`.** On the heavy image the archive is written to `/app/repo/backend-extensions`, which the supervisor ignores. The rebuild then answers `these inputs are already built, nothing to do` and the resulting binary does not carry the extension.

Expect around ten minutes for the first build, then a matter of seconds while the compilation cache stays warm. The extension must be rebuilt on every panel upgrade, and the panel ships weekly.

### Setting the shared secret

**From ClientXCMS, in one command, with nothing to type here:**

```sh
php artisan calagopus:sso
```

It draws a random secret, sets it on the panel through the route below, then keeps it encrypted on its side. This is the normal route, and it guarantees both halves match.

The API key ClientXCMS uses must carry the **`ssotickets.manage`** permission, which this extension adds to the panel once installed.

If you drive the panel on its own, the route exists:

```sh
curl -X PUT "https://panel.example.net/api/admin/ssotickets/secret" \
  -H "Authorization: Bearer <key carrying ssotickets.manage>" \
  -H "Content-Type: application/json" \
  -d '{"secret":"<at least 32 characters>"}'
```

The secret is stored encrypted in the extension settings. Rotating it means running `php artisan calagopus:sso` again: no rebuild, no restart.

### Checking it works

In ClientXCMS, open the server page and run **Test connection**: it states plainly whether single sign-on is configured. Then, from a customer account holding an active service, click "Open the panel": you should land signed in, with no login page.

On panel startup, the `clientxcms sso extension loaded` line confirms the extension is in the running binary.

### When it does not work

| Symptom | Likely cause |
|---|---|
| `extension management is only available in the official heavy container` | Panel on the standard image, see Requirements |
| The panel loops on `/app/binaries is missing or is not a directory` | The four volumes are not mounted |
| `these inputs are already built, nothing to do` and no extension | Installed through `panel-rs extensions add`, see above |
| The rebuild fails | Build logs are served by `GET /api/admin/extensions/manage/logs`, and displayed under **Admin > Extensions** |
| The customer lands on the panel login page | Missing or mismatched secret: run `php artisan calagopus:sso` again |
| ClientXCMS reports a missing permission | The API key lacks `ssotickets.manage` |

To find your panel version: it is shown in the panel admin area, and readable through `GET /api/admin/system/overview`.

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

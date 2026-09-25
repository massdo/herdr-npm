# Compiler, installer et publier herdr-npm

## Prérequis

- Herdr 0.9.1 ou une version compatible ;
- Rust 1.89 pour compiler (inutile quand l'installation retient un binaire
  précompilé vérifié) ;
- macOS ou Linux ;
- `npm` ou `pnpm` disponible dans le `PATH`.

## Compiler le plugin localement

Depuis la racine du dépôt :

```sh
cd /chemin/vers/herdr-npm
sh scripts/build.sh
```

Le script lance une compilation Rust optimisée et verrouillée. Le binaire produit se trouve ici :

```text
target/release/herdr-npm
```

## Installer le checkout local dans Herdr

Toujours depuis la racine du dépôt :

```sh
herdr plugin link "$PWD" --enabled
```

Cette commande enregistre le dossier local du plugin. Il n'est donc pas nécessaire de recopier le binaire ailleurs.

Pour vérifier que Herdr le reconnaît :

```sh
herdr plugin list
```

## Ouvrir le plugin

Ouvrez dans Herdr un projet qui contient un `package.json`, puis exécutez :

```sh
herdr plugin action invoke herdr-npm.toggle
```

La colonne affiche les scripts npm ou pnpm du paquet courant. Dans un workspace (monorepo) déclaré par `pnpm-workspace.yaml` ou par le champ `workspaces` du `package.json` racine, elle affiche un groupe par paquet, et chaque script se lance dans le dossier de son paquet ; la section « Workspaces / monorepos » du README détaille ce mode. Utilisez les flèches ou `j` et `k` pour sélectionner un script, puis `Entrée` pour le lancer. Appuyez sur `q` pour fermer la colonne.

Pour ajouter un raccourci, placez l'une de ces configurations dans la configuration personnelle de Herdr :

```toml
# macOS
[[keys.command]]
key = "cmd+shift+s"
type = "plugin_action"
command = "herdr-npm.toggle"

# Linux, après la touche préfixe Herdr
[[keys.command]]
key = "prefix+shift+s"
type = "plugin_action"
command = "herdr-npm.toggle"
```

Après une modification du code, fermez la colonne, recompilez, puis rouvrez-la :

```sh
sh scripts/build.sh
herdr plugin action invoke herdr-npm.toggle
```

## Vérifier avant publication

```sh
sh scripts/check.sh all
sh scripts/e2e.sh
```

Une fois le commit poussé sur GitHub, `sh scripts/install-smoke.sh <SHA>`
l'installe dans un profil Herdr jetable, sur un paquet simple puis sur un
workspace npm.

## Réglages du dépôt public

Le dépôt `massdo/herdr-npm` est public. Ces réglages ont été faits une fois ;
ils se vérifient dans les réglages GitHub et ne sont pas à refaire à chaque
release :

- signalement privé des vulnérabilités activé, vers lequel renvoie `SECURITY.md` ;
- secret scanning et protection des push activés ;
- alertes et correctifs de sécurité Dependabot activés ; mises à jour de
  versions mensuelles configurées dans `.github/dependabot.yml` ;
- branche `main` protégée : pull request obligatoire, cinq checks requis
  (`offline` et `recipe` sur macOS et Ubuntu, `secrets`), historique linéaire,
  règles appliquées aussi aux administrateurs ;
- suppression automatique des branches après fusion ;
- immutabilité des releases activée :
  `gh api repos/massdo/herdr-npm/immutable-releases` renvoie `"enabled": true`.

Le marketplace Herdr indexe automatiquement les dépôts publics portant le topic GitHub `herdr-plugin`. L'actualisation peut prendre environ trente minutes.

## Publier une release

1. Alignez la version dans `Cargo.toml`, `Cargo.lock` et `herdr-plugin.toml`,
   fusionnez sur `main` et attendez que CI et E2E soient verts sur ce commit.
   Le workflow `release` ne relance pas les E2E Herdr.
2. Les notes de release reprennent les titres des pull requests fusionnées
   depuis la release précédente. Retitrez celles qui ne sont pas lisibles pour
   le public, puis prévisualisez les notes :

   ```sh
   gh api repos/massdo/herdr-npm/releases/generate-notes \
     -f tag_name=v0.2.0 -f target_commitish=<SHA> -f previous_tag_name=v0.1.0 \
     --jq .body
   ```

3. Créez le tag sur ce commit validé, puis poussez-le :

   ```sh
   git tag -a v0.2.0 <SHA> -m v0.2.0
   git push origin v0.2.0
   ```

4. Le workflow `release` refuse un tag qui diverge de ces trois versions,
   relance `sh scripts/check.sh all` sur macOS et Linux, puis compile les trois
   binaires sur leurs architectures. Il assemble `herdr-npm-<triple>`,
   `SHA256SUMS` et `SOURCE_COMMIT` depuis ce même commit, crée une release
   draft, y charge les fichiers, les retélécharge pour les comparer, vérifie
   que le tag désigne toujours ce commit, puis publie. Relancer le workflow
   reprend un draft ; une release déjà publiée n'est jamais écrasée.
5. Validez l'installation depuis la release publique sur macOS arm64 et sur
   Linux x86_64, sur des machines dont `/usr/bin`, `/bin`, `/usr/sbin` et
   `/sbin` ne contiennent pas Rust, puis conservez la sortie comme preuve :

   ```sh
   sh scripts/install-smoke.sh v0.2.0 --prebuilt
   ```

   Le script construit un environnement jetable sans Rust et vérifie le
   commit, le hash, le manifeste, le message de téléchargement et l'absence
   de compilation. Il ouvre ensuite la colonne sur un paquet simple, puis sur
   un workspace npm depuis sa racine et depuis un membre, et lance chaque fois
   un script témoin. Ses journaux restent dans `/tmp/hni-diag.*`.

Les utilisateurs installent une version publiée, ce qui est recommandé :

```sh
herdr plugin install massdo/herdr-npm --ref v0.2.0 --yes
```

Ils peuvent aussi installer la branche par défaut, qui se compile depuis les
sources quand aucune release ne correspond à son commit :

```sh
herdr plugin install massdo/herdr-npm --yes
```

Herdr récupère alors le dépôt et exécute `scripts/fetch-or-build.sh`, déclaré
dans `herdr-plugin.toml`. Sur macOS arm64, macOS x86_64 et Linux x86_64, ce
script installe le binaire publié quand la release a été construite depuis ce
commit exact et que `SOURCE_COMMIT` et `SHA256SUMS` le confirment. Dans tous
les autres cas, il compile depuis les sources avec `scripts/build.sh` et
indique la raison sur stderr. Herdr 0.9.1 n'affiche la sortie du build qu'en
cas d'échec ; pour conserver ces lignes, définissez `HERDR_NPM_BUILD_LOG`
avec le chemin d'un fichier avant l'installation. Herdr active ensuite le
plugin pour l'utilisateur.

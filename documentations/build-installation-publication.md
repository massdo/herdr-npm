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

La colonne affiche les scripts npm ou pnpm du paquet courant. Utilisez les flèches ou `j` et `k` pour sélectionner un script, puis `Entrée` pour le lancer. Appuyez sur `q` pour fermer la colonne.

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

## Rendre le plugin public

1. Auditez les fichiers, tout l'historique Git, les branches et tags, ainsi que
   les titres, descriptions, commentaires et références des pull requests.
   Vérifiez les secrets et les informations personnelles des auteurs et
   committers. Retirer une donnée du dernier commit ne l'efface pas de
   l'historique. Un force-push ne purge pas non plus les anciennes références
   de pull requests ni les vues mises en cache sur GitHub ; voir la
   [procédure GitHub de suppression des données sensibles](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/removing-sensitive-data-from-a-repository).
2. Faites valider séparément le traitement de l'historique et le passage en
   public. Vérifiez `SECURITY.md`, la description GitHub et la présence de
   `herdr-plugin.toml` sur la branche par défaut. Activez les alertes Dependabot
   dès maintenant si elles sont disponibles ; leur état se vérifie séparément
   de `security_and_analysis`.
3. Rendez le dépôt GitHub `massdo/herdr-npm` public uniquement après cette
   validation.
4. Activez et vérifiez le signalement privé des vulnérabilités, puis mettez à
   jour le paragraphe de disponibilité dans `SECURITY.md`. Vérifiez également
   le secret scanning et la protection des push dans les réglages GitHub.
5. Ajoutez le topic GitHub `herdr-plugin` pour demander l'indexation publique.
   GitHub autorise les topics sur les dépôts privés ; c'est l'indexation du
   marketplace qui nécessite un dépôt public.
6. Publiez une release (section suivante), puis mettez à jour les versions
   prises en charge dans `SECURITY.md`.

Le marketplace Herdr indexe automatiquement les dépôts publics portant le topic `herdr-plugin`. L'actualisation peut prendre environ trente minutes.

## Publier une release

1. Une seule fois, avant la première publication : dans les réglages GitHub du
   dépôt, section « Releases », cochez **Enable release immutability**.
   L'immutabilité ne s'applique qu'aux releases publiées ensuite. Vérifiez que
   `gh api repos/massdo/herdr-npm/immutable-releases` renvoie
   `"enabled": true`.
2. Alignez la version dans `Cargo.toml`, `Cargo.lock` et `herdr-plugin.toml`,
   fusionnez sur `main` et attendez une CI verte.
3. Créez le tag sur ce commit validé, puis poussez-le :

   ```sh
   git tag -a v0.1.0 <SHA> -m v0.1.0
   git push origin v0.1.0
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
   sh scripts/install-smoke.sh <SHA du tag> --prebuilt
   ```

   Le script construit un environnement jetable sans Rust et vérifie le
   commit, le hash, le manifeste, le message de téléchargement et l'absence
   de compilation, puis ouvre la colonne, lance `hello` et la referme.

Les utilisateurs pourront installer la branche par défaut avec :

```sh
herdr plugin install massdo/herdr-npm --yes
```

Ils pourront aussi installer une version publiée, ce qui est recommandé :

```sh
herdr plugin install massdo/herdr-npm --ref v0.1.0 --yes
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

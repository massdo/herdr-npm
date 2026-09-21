# Compiler, installer et publier herdr-npm

## Prérequis

- Herdr 0.9.1 ou une version compatible ;
- Rust 1.89 ;
- macOS ou Linux ;
- `npm` ou `pnpm` disponible dans le `PATH`.

## Compiler le plugin localement

Depuis la racine du dépôt :

```sh
cd /chemin/vers/herdr-npm
sh plugins/herdr-npm/scripts/build.sh
```

Le script lance une compilation Rust optimisée et verrouillée. Le binaire produit se trouve ici :

```text
plugins/herdr-npm/target/release/herdr-npm
```

## Installer le checkout local dans Herdr

Toujours depuis la racine du dépôt :

```sh
herdr plugin link "$PWD/plugins/herdr-npm" --enabled
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
sh plugins/herdr-npm/scripts/build.sh
herdr plugin action invoke herdr-npm.toggle
```

## Vérifier avant publication

```sh
sh plugins/herdr-npm/scripts/check.sh all
sh plugins/herdr-npm/scripts/e2e.sh
```

## Rendre le plugin public

1. Rendez le dépôt GitHub `massdo/herdr-npm` public.
2. Ajoutez le topic GitHub `herdr-plugin` au dépôt.
3. Vérifiez que `plugins/herdr-npm/herdr-plugin.toml` se trouve sur la branche par défaut.
4. Créez un tag, par exemple `v0.1.0`, afin de désigner une version stable.

Le marketplace Herdr indexe automatiquement les dépôts publics portant le topic `herdr-plugin`. L'actualisation peut prendre environ trente minutes.

Les utilisateurs pourront installer la branche par défaut avec :

```sh
herdr plugin install massdo/herdr-npm/plugins/herdr-npm --yes
```

Ils pourront aussi installer une version précise avec :

```sh
herdr plugin install massdo/herdr-npm/plugins/herdr-npm --ref v0.1.0 --yes
```

Herdr récupère alors le dépôt, exécute la commande de compilation déclarée dans `herdr-plugin.toml`, puis active le plugin pour l'utilisateur.


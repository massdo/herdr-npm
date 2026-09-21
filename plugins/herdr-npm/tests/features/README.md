# Tests herdr-npm

Depuis la racine du dépôt :

```sh
sh plugins/herdr-npm/scripts/check.sh all
sh plugins/herdr-npm/scripts/e2e.sh
sh plugins/herdr-npm/scripts/install-smoke.sh <commit>
```

- Cucumber vérifie les règles de catalogue, gestionnaire, navigation, lancement,
  erreurs et toggle. Un clic sur la gouttière d'icône lance ; un clic sur le nom,
  la commande ou la fin de ligne sélectionne seulement. Le système de fichiers et
  le rendu TestBackend sont réels ; le port Herdr est simulé. Chaque scénario
  possède un dossier temporaire nettoyé.
- Les tests Rust ciblent les limites techniques : transport socket, verrous,
  géométrie, résolution des fichiers et arguments reçus par les vrais shells.
- Le parcours Python E2E pilote un seul Herdr 0.9.1 isolé et son client PTY :
  cohabitation avec l'explorateur (ouverture automatique désactivée), clic unique, clavier, argv/cwd, focus,
  arrêt du processus, sortie conservée, raccourci, fermeture et redémarrage.
- Le smoke test installe le plugin depuis GitHub au commit demandé.

Les tests offline nécessitent Rust 1.89, Python 3, npm et pnpm ; les E2E demandent
également Herdr 0.9.1 et un accès réseau pour installer l'explorateur épinglé.
La recette crée une session et une configuration temporaires ; elle les supprime
à la fin et affiche les panneaux en cas d'échec.

`check.sh catalog`, `toggle` et `run` filtrent les scénarios par tag.
`check.sh harness` exécute seulement Cucumber. Les steps manquants et les
sélections vides échouent ; le résumé Cucumber fait foi pour le nombre exécuté.
Le tag `@v1_1_icon` isole les scénarios de lancement par gouttière d'icône.

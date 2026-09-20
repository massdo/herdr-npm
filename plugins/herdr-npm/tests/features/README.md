# Features herdr-npm V1

Spécifications exécutables de la V1. Le harnais Cucumber (`tests/features.rs`, `scripts/check.sh`) est livré avec le plugin : les steps manquants échouent, `@e2e` est exclu par défaut.

## Fixtures par défaut

Chaque scénario déclare ses propres fichiers. Rien n’est hérité d’un scénario précédent.

`/work/app/package.json` quand le catalogue nominatif est cité :

```json
{
  "name": "app",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "test": "vitest run"
  }
}
```

`/work/other/package.json` quand un second package est cité :

```json
{
  "name": "other",
  "scripts": {
    "start": "node server.js",
    "lint": "eslint ."
  }
}
```

Catalogue long : `s1` … `s40` dans `/work/app`, commandes `echo sN`.

TestBackend par défaut : 32 colonnes **extérieures** × 24 lignes. Le message `Terminal too small` s’applique en dessous de 12 colonnes **intérieures** ou 4 lignes.

Cwd d’origine : `foreground_cwd` du pane capturé par le toggle. Repli : `cwd` de démarrage du pane, avec le message `Current directory unavailable; using pane start directory`.

## Tags et lots

| Tag | Fichiers | Lot responsable des steps verts |
|---|---|---|
| `@toggle` | `toggle_sidebar.feature` | `toggle` (géométrie d’ouverture vide : `squelette`) |
| `@catalog` | `list_scripts.feature`, `detect_package_manager.feature` | `catalogue` |
| `@run` | `run_script.feature` | `exécution` |
| `@e2e` | scénarios marqués en plus | `recette E2E` |

Sans filtre, le harnais sélectionne `not @e2e`. Un `--tags` explicite remplace ce défaut.

## Comptage après expansion

Baseline avant ce lot : 39 déclarations / 55 cas, dont 6 `@e2e`.

Après alignement V1 :

| Fichier | Déclarations | Cas | dont `@e2e` |
|---|---:|---:|---:|
| `toggle_sidebar.feature` | 20 | 20 | 3 |
| `list_scripts.feature` | 36 | 44 | 0 |
| `detect_package_manager.feature` | 4 | 18 | 0 |
| `run_script.feature` | 23 | 37 | 4 |
| **Total** | **83** | **119** | **7** |

Hors `@e2e` : 76 déclarations, 112 cas. Le 7e `@e2e` est la procédure de redémarrage (pane restauré inerte). Les 6 cas historiques (ancrage explorer, `q`, arrêt du script, sortie conservée, `q` transmis, CLI) restent.

Les assertions de messages inspectent les cellules rendues par TestBackend. Les lancements utilisent `flush_intents`, comme la TUI réelle. Le mode `check.sh harness` échoue si la suite métier échoue.

Le harnais recomptera à l’exécution. Ce tableau est le compte lu dans les fichiers livrés.

## Table règle → scénario → lot

| Règle V1 | Scénario | Lot |
|---|---|---|
| Raccourci unique `herdr-npm.toggle` ; pas de sidebar reconnue → ouverture et focus | Opening the sidebar in a tab that has none | `toggle` / squelette `squelette` |
| Largeur préférée 32 colonnes extérieures ; hauteur de la cible | Opening the sidebar in a tab that has none | `squelette` |
| Sidebar reconnue → fermeture depuis son pane ; focus rendu à la cible du split | Closing the sidebar from the sidebar itself | `toggle` |
| Fermeture depuis un autre pane du même onglet ; le focus ne bouge pas | Closing the sidebar from another pane of the tab | `toggle` |
| Deux toggles sérialisés : ouvert puis fermé | Two toggles open then close | `toggle` |
| Aucun effet sur les autres onglets | The sidebar of another tab is never touched | `toggle` |
| Un token d’un autre onglet n’est pas une sidebar locale | A token in another tab is not a sidebar of the focused tab | `toggle` |
| Label `npm` seul : pas une reconnaissance | A pane labelled npm without the token is not the sidebar | `toggle` |
| Verrou `launcher.lock` ; deux appels concurrents n’ouvrent pas deux sidebars | Concurrent toggles are serialised to open then close | `toggle` |
| Snapshot illisible → erreur, aucune mutation | An unreadable pane list is an error without mutation | `toggle` |
| Contexte d’origine absent → erreur, aucune mutation | A missing origin context is an error without mutation | `toggle` |
| Contexte d’origine disparu → erreur, aucune mutation | A changed origin context is an error without mutation | `toggle` |
| Plusieurs sidebars reconnues → erreur, aucune mutation | Several recognised sidebars are an error without mutation | `toggle` |
| Aucune cible de travail → erreur, aucune mutation | No working-pane target is an error without mutation | `toggle` |
| Erreur de transport incertaine : pas de retry, inspection manuelle | An uncertain transport error is never retried automatically | `toggle` |
| Capturer workspace/tab/pane avant les I/O | The origin context is captured before any I/O | `toggle` |
| Layout déjà découpé verticalement : garder la hauteur de la cible | A vertically split layout keeps the target pane height | `squelette` / `toggle` |
| Le toggle ne lit pas package.json | Toggle does not read package.json | `toggle` |
| Explorer herdr-sidebar exclu ; splitter le pane de travail à sa droite | Docking next to the herdr-sidebar explorer | `recette E2E` |
| `q` ferme le pane TUI | Closing the sidebar with q | `squelette` + preuve `recette E2E` |
| Après redémarrage : pane inerte, fermeture manuelle, puis ouverture ; pas de remplacement auto | A restored pane after restart is inert until closed by hand | `recette E2E` |
| Lister tous les scripts, icône ▶, en-tête name + gestionnaire, sélection initiale sur le premier | Listing every script of the package | `catalogue` |
| Ordre de déclaration, y compris pre/post | The scripts keep their declaration order | `catalogue` |
| Remonter au premier package.json | Finding the package.json from a sub-directory | `catalogue` |
| Le plus proche gagne | The nearest package.json wins | `catalogue` |
| Package imbriqué invalide : ne pas lire le parent | An invalid nested package is not skipped for its parent | `catalogue` |
| Package imbriqué vide : ne pas lire le parent | An empty nested package is not skipped for its parent | `catalogue` |
| Racine figée si le cwd change | The project root is frozen for the lifetime of the sidebar | `catalogue` |
| Catalogue figé si package.json change sur disque | The listed scripts stay frozen if package.json changes on disk | `catalogue` |
| Prochaine ouverture relit le cwd d’origine | The project root is resolved again at the next opening | `catalogue` |
| `foreground_cwd` prioritaire | The live foreground cwd is preferred over the start directory | `catalogue` |
| Repli `cwd` + message fixé | The start directory is used when the live cwd is missing | `catalogue` |
| Ni live ni start → `Cannot determine project directory` | Neither live cwd nor start directory can be used | `catalogue` |
| Aucun fichier → `No package.json found` | No package.json anywhere above the origin pane | `catalogue` |
| JSON invalide → `package.json is not valid JSON` | An invalid package.json does not crash the sidebar | `catalogue` |
| Racine non objet : même message | A package.json whose root is not an object | `catalogue` |
| Lecture refusée → `Cannot read package.json` | A package.json that cannot be read | `catalogue` |
| Champ `scripts` absent → `This package.json has no scripts` | A package.json without a scripts field | `catalogue` |
| Objet `scripts` vide : même message | An empty scripts field | `catalogue` |
| `scripts` n’est pas un objet de chaînes | scripts must be an object of strings | `catalogue` |
| Commande chaîne vide : listée et transmissible | An empty command string is still listed / still forwarded | `catalogue` / `exécution` |
| `name` absent, vide ou non chaîne → nom du dossier | missing / empty / non-string package name | `catalogue` |
| ↑↓ et j/k sans boucle aux extrémités | Navigating the script list stays inside the bounds | `catalogue` |
| 40 scripts accessibles ; le catalogue sélectionne, il ne lance pas | A long list scrolls instead of hiding scripts | `catalogue` |
| Ellipsis sur nom/commande trop longs ; icône conservée | Text too long for the column is cut with an ellipsis | `catalogue` |
| Largeur terminal Unicode, pas de coupe au milieu d’un caractère large | Wide characters use terminal display width | `catalogue` |
| Pied = commande du sélectionné, une ligne, aide h/l | The selected script shows its full command on one footer line | `catalogue` |
| h/l défilent le pied d’une cellule, sans changer le script | h and l scroll the footer command without changing the selection | `catalogue` |
| Offset pied remis à zéro au changement de sélection | Changing the selection resets the footer scroll | `catalogue` |
| < 12 colonnes intérieures ou 4 lignes → `Terminal too small` | A terminal that is too small shows a blocking message | `catalogue` |
| Agrandissement : rendu normal | Enlarging a too-small terminal restores the catalogue | `catalogue` |
| Clic après scroll/resize : coordonnées du rendu | Mouse coordinates follow scrolling and resizing | `catalogue` |
| Clic gauche sur toute la ligne : sélection + une intention de lancement | A left click on a visible script row… | `catalogue` (intention) / `exécution` (effet) |
| Relâchement et mouvement : rien | Mouse release and movement do not emit a run intent | `catalogue` |
| En-tête, vide, pied : ni sélection ni lancement | Clicking outside a script row… | `catalogue` / `exécution` |
| `packageManager` npm/pnpm, avec ou sans version, prime sur le lockfile | Picking between npm and pnpm (premier groupe d’Examples) | `catalogue` |
| Lockfiles du dossier du package seulement : pnpm puis npm puis npm | Picking… (groupe lockfile / fallback) | `catalogue` |
| yarn/bun déclarés → npm, sans avertissement, même avec pnpm-lock | Picking… (yarn/bun) + An unsupported package manager… | `catalogue` |
| Champ inconnu, vide ou mal typé → règle des lockfiles | Picking… (unknown or mistyped) | `catalogue` |
| Lockfile parent ignoré | A lockfile in a parent directory is ignored | `catalogue` |
| Lockfile d’un package ancêtre ignoré pour un nested package | Lockfiles are read only in the package directory | `catalogue` |
| Entrée lance dans un onglet `focus: false`, cwd package, workspace du catalogue | Running the selected script with Enter | `exécution` |
| Un Mouse Down lance une fois | A single left click… / A single mouse down is enough | `exécution` |
| Sidebar garde le focus et le catalogue | The new tab does not steal the focus | `exécution` |
| Chaque action crée un onglet ; pas de recyclage | Every launch opens its own tab / Firing several scripts | `exécution` |
| Label d’onglet = commande invoquée | The new tab is labelled after the command | `exécution` |
| pnpm run avec le nom littéral | Running with pnpm | `exécution` |
| cwd = racine du package figée | The script runs in the package directory… | `exécution` |
| Nom = un argument littéral (espaces, apostrophe, métacaractères, tiret initial) | The script name is forwarded as one literal argument | `exécution` |
| 40e script : sélection au catalogue, lancement ici | The 40th selected script can be launched | `exécution` |
| Échec de `tab.create` → aucun envoi | A failed tab creation sends no input | `exécution` |
| Timeout / ack manquant → `Script launch not confirmed` + id connu, pas de retry | A timeout after tab creation does not retry | `exécution` |
| Réponse sans `root_pane` utilisable | A missing root pane after creation does not retry | `exécution` |
| Changement de focus global : workspace/cwd inchangés | A global focus change does not retarget the launch | `exécution` |
| Fermer l’onglet termine le script ordinaire de recette | Closing a script tab kills the script | `recette E2E` |
| Fin normale : onglet et sorties lisibles | The script tab stays open once the script has ended | `recette E2E` |
| `q` dans l’onglet script n’est pas intercepté | The sidebar does not intercept keys typed in a script tab | `recette E2E` |
| Pilotage CLI `herdr pane send-keys` | Driving the sidebar from the CLI | `recette E2E` |

Hors V1 (donc absents) : workspaces npm, yarn/bun comme gestionnaires, Windows, heartbeat, remplacement automatique, stop/restart depuis la sidebar, marketplace, binaires précompilés.

## Raccourcis à documenter à l’installation

Les deux bindings appellent la même action, via `[[keys.command]]`, `type = "plugin_action"`, `command = "herdr-npm.toggle"` :

- macOS / Ghostty : `cmd+shift+s`
- Linux : `prefix+shift+s` (préfixe Herdr puis Shift+S)

Les tests n’écrivent pas la configuration personnelle ; profil de recette séparé (`recette E2E`).

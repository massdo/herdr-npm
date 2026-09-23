# Audit sécurité et CI GitHub — 23 septembre 2026

Périmètre : dépôt privé `massdo/herdr-npm`, branche `main` à
`746aa12a20369cedf376842fc6d6deba899b3d8f`. Cet audit confronte les
fichiers du dépôt aux réponses de l'API GitHub et aux exécutions Actions ; il
n'a pas modifié les réglages du dépôt.

## Verdict

Les deux workflows ont réussi sur macOS et Ubuntu avant les fusions examinées,
puis sur `main`. Cependant, **aucun contrôle observé n'impose leur réussite
avant une fusion** : l'API indique `protected: false` pour `main` et ne retourne
aucun status check requis. La consultation des rulesets et de la protection
détaillée renvoie HTTP 403 avec « Upgrade to GitHub Pro or make this repository
public to enable this feature ». On ne peut donc pas certifier le contenu d'un
éventuel ruleset par cette API, mais aucun mécanisme bloquant n'a été confirmé.

## Sécurité

| Contrôle | État constaté | Preuve / limite |
| --- | --- | --- |
| Politique `SECURITY.md` | Présente sur `main`. Elle couvre le code courant, explique le risque d'exécuter des scripts npm/pnpm et indique correctement que le dépôt est privé et que le formulaire de signalement n'est pas encore disponible. | Fichier `SECURITY.md` au commit audité ; `GET /repos/massdo/herdr-npm` renvoie `private: true`. |
| Signalement privé des vulnérabilités par des tiers | Indisponible actuellement. | GitHub réserve [cette fonction aux dépôts publics](https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/configure-vulnerability-reporting/configure-for-a-repository). L'URL mentionnée dans `SECURITY.md` n'est pas encore un canal utilisable par des chercheurs externes. |
| Alertes Dependabot | Activées ; zéro alerte retournée lors de l'audit. | `GET /repos/massdo/herdr-npm/vulnerability-alerts` → HTTP 204 ; `GET /repos/massdo/herdr-npm/dependabot/alerts` → liste vide. Zéro alerte ne prouve pas l'absence de vulnérabilités. |
| Secret scanning du dépôt | Désactivé. | `GET /repos/massdo/herdr-npm/secret-scanning/alerts` → HTTP 404, message explicite « Secret scanning is disabled on this repository. » |
| Push protection du dépôt | Aucun blocage de secrets confirmé ; indisponible tant que la protection des secrets du dépôt n'est pas activée. | Le bloc `security_and_analysis` de `GET /repos/massdo/herdr-npm` vaut `null`, donc l'état individuel n'est pas lisible par cette réponse. Selon [GitHub](https://docs.github.com/en/code-security/concepts/secret-security/push-protection), la push protection du dépôt exige GitHub Secret Protection et est désactivée par défaut. Ce verdict est une déduction à partir du secret scanning désactivé et de cette dépendance, pas une lecture directe du commutateur. La protection personnelle des push vers les dépôts publics est distincte. |

## Workflows et exécutions

| Moment | Déclenchement et couverture | Exécutions observées |
| --- | --- | --- |
| Avant fusion | `ci.yml` et `e2e.yml` se déclenchent sur `pull_request` sans filtre de branche. Chaque workflow exécute une matrice `macos-latest` / `ubuntu-latest`. | PR [#2](https://github.com/massdo/herdr-npm/pull/2) : [CI](https://github.com/massdo/herdr-npm/actions/runs/35600986971) et [E2E](https://github.com/massdo/herdr-npm/actions/runs/35600986981), quatre jobs réussis avant la fusion du 21 septembre à 12:49 UTC. PR [#1](https://github.com/massdo/herdr-npm/pull/1) : [CI](https://github.com/massdo/herdr-npm/actions/runs/35559348505) et [E2E](https://github.com/massdo/herdr-npm/actions/runs/35559348509) réussis. Des échecs antérieurs sont aussi visibles, par exemple [E2E](https://github.com/massdo/herdr-npm/actions/runs/35589612575) et [CI](https://github.com/massdo/herdr-npm/actions/runs/35586569419) sur la PR #2. |
| Après fusion sur `main` | Les deux workflows se déclenchent sur `push` vers `main`. | Après la PR #2 : [CI](https://github.com/massdo/herdr-npm/actions/runs/35601766133) et [E2E](https://github.com/massdo/herdr-npm/actions/runs/35601765796) réussis. Au dernier commit audité : [CI](https://github.com/massdo/herdr-npm/actions/runs/35734259648) et [E2E](https://github.com/massdo/herdr-npm/actions/runs/35734259423) réussis, avec les quatre jobs et leurs étapes en succès. |

`scripts/check.sh all` lance `cargo fmt --check`, `cargo clippy --all-targets
-- -D warnings` et `cargo test` (tests Rust et Cucumber). `e2e.yml` installe
Herdr 0.9.1 avec une somme SHA-256 contrôlée, lance les parcours E2E standard
et V1.1, puis `scripts/install-smoke.sh` depuis le SHA testé. Les jobs E2E
utilisent un profil Herdr isolé. Les workflows déclenchent également les push
vers l'ancienne branche `feat/herdr-npm-v1` ; cette branche n'est plus nécessaire
pour couvrir les PR.

Les jobs Actions sont des **signaux**, pas des portes de fusion :
`GET /repos/massdo/herdr-npm/branches/main` renvoie `protected: false` et
`required_status_checks: {checks: [], contexts: [], enforcement_level: "off"}`.
Les endpoints de rulesets et de protection de branche retournent HTTP 403
avec le message de limitation de plan cité plus haut. Selon la
[documentation GitHub](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches),
la protection des branches privées requiert une offre adaptée.

## Corrections proposées, par priorité

1. **P0 — avant publication :** terminer la revue de l'historique et obtenir la
   validation du passage en public prévue par le guide de publication. Une fois
   le dépôt public, activer et tester le signalement privé depuis un compte
   externe, puis remplacer dans `SECURITY.md` la mention « currently private »
   par un canal réellement disponible. Prévoir un canal privé alternatif si la
   publication est reportée.
2. **P1 — fusion :** rendre les quatre jobs `offline (macos-latest)`,
   `offline (ubuntu-latest)`, `recipe (macos-latest)` et
   `recipe (ubuntu-latest)`, ainsi que le nouveau job `secrets`, obligatoires
   sur `main` via protection de branche ou ruleset, après passage en public ou
   changement de plan. Exiger une PR et vérifier avec une PR d'essai en échec
   que le bouton de fusion est bloqué. D'ici là, la réussite des jobs dépend
   d'une discipline manuelle.
3. **P1 — secrets :** lors du changement de visibilité ou de plan, vérifier le
   secret scanning et activer la push protection du dépôt si la fonction est
   disponible. Pour le dépôt privé actuel, utiliser un contrôle de secrets
   indépendant dans la CI si une protection avant publication est nécessaire ;
   vérifier séparément le périmètre des offres GitHub disponibles.
4. **P2 — chaîne CI :** épingler les actions tierces à des SHA vérifiés et la
   version exacte de pnpm (actuellement `pnpm@10`), puis déclarer
   `permissions: contents: read` dans les workflows. Le réglage actuel des
   permissions par défaut est déjà `read`. Retirer le déclencheur de push vers
   `feat/herdr-npm-v1` lorsqu'il n'est plus utile.

Sources locales : `SECURITY.md`, `.github/workflows/ci.yml`,
`.github/workflows/e2e.yml`, `scripts/check.sh`, `scripts/e2e.sh` et
`scripts/install-smoke.sh` au commit audité. Les réponses API et les exécutions
GitHub ont été consultées le 23 septembre 2026 avec un compte administrateur
du dépôt. Les liens Actions sont privés et nécessitent cet accès.

## Vérification complémentaire avant publication

Le 23 septembre, un scan local Gitleaks 8.30.1 sur les 60 commits accessibles
par `main` et les références GitHub des PR #1 et #2 n'a produit aucun
signalement. Les titres et corps des PR, ainsi que les commentaires et avis
associés, ont été examinés automatiquement pour les adresses, chemins locaux,
IP et marqueurs de credentials : aucun de ces motifs n'a été trouvé. Ce scan
n'est pas une garantie d'absence de données sensibles.

L'historique de `main` ne contient **aucune** adresse Gmail (37 commits au
contrôle du 23 septembre). Une adresse Gmail figure dans les métadonnées
d'auteur et de committer de 27 commits atteignables par les anciennes
références GitHub `refs/pull/1/head` et `refs/pull/2/head`. Ces références
seraient exposées avec les PR si le dépôt actuel devenait public. Réécrire
`main` ne les effacerait pas ; [GitHub décrit les limites des réécritures
d'historique](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/removing-sensitive-data-from-a-repository),
notamment pour les références de PR et les vues mises en cache. Le rapport ne
reproduit pas l'adresse.

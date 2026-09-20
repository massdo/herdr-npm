# Pure domain rule, exercised through the sidebar.
# Precedence: recognised packageManager field (npm or pnpm) > lockfiles in the
# package directory only (pnpm-lock.yaml then package-lock.json) > npm.
# yarn and bun declared in packageManager select npm with no warning.
# Unknown, missing or mistyped fields fall through to the lockfile rule.

@catalog
Feature: Detect the package manager of the project
  As a developer using pnpm on some repos and npm on others
  I want the sidebar to pick the right package manager on its own
  So that a script is never run with the wrong one

  Scenario Outline: Picking between npm and pnpm
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command |
      | build | tsc     |
    And the package.json <packageManager>
    And the project contains <lockfiles>
    And the origin pane foreground cwd is "/work/app"
    When the sidebar opens
    Then the detected package manager is "<manager>"
    And the sidebar header shows "<manager>"

    Examples: the packageManager field wins over the lockfile
      | packageManager                 | lockfiles                            | manager |
      | declares "pnpm@9.0.0"          | a package-lock.json                  | pnpm    |
      | declares "npm@10.0.0"          | a pnpm-lock.yaml                     | npm     |
      | declares "pnpm"                | a package-lock.json                  | pnpm    |
      | declares "npm"                 | a pnpm-lock.yaml                     | npm     |

    Examples: the lockfile decides when the field is absent
      | packageManager                 | lockfiles                            | manager |
      | has no packageManager field    | a pnpm-lock.yaml                     | pnpm    |
      | has no packageManager field    | a package-lock.json                  | npm     |
      | has no packageManager field    | both lockfiles                       | pnpm    |

    Examples: npm is the fallback
      | packageManager                 | lockfiles                            | manager |
      | has no packageManager field    | no lockfile                          | npm     |
      | declares "yarn@4.0.0"          | a yarn.lock                          | npm     |
      | declares "bun@1.1.0"           | a bun.lockb                          | npm     |
      | declares "yarn@4.0.0"          | a pnpm-lock.yaml                     | npm     |
      | declares "bun@1.1.0"           | a package-lock.json                  | npm     |

    Examples: unknown or mistyped fields fall through to lockfiles
      | packageManager                 | lockfiles                            | manager |
      | declares "deno@1.0.0"          | a pnpm-lock.yaml                     | pnpm    |
      | has a non-string packageManager| a package-lock.json                  | npm     |
      | has an empty packageManager    | a pnpm-lock.yaml                     | pnpm    |

  Scenario: An unsupported package manager is not announced to the user
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command |
      | build | tsc     |
    And the package.json declares "yarn@4.0.0"
    And the origin pane foreground cwd is "/work/app"
    When the sidebar opens
    Then the detected package manager is "npm"
    And no warning is shown

  Scenario: A lockfile in a parent directory is ignored
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command |
      | build | tsc     |
    And the package.json has no packageManager field
    And the project contains no lockfile
    And "/work/pnpm-lock.yaml" exists
    And the origin pane foreground cwd is "/work/app"
    When the sidebar opens
    Then the detected package manager is "npm"

  Scenario: Lockfiles are read only in the package directory
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command |
      | build | tsc     |
    And a nested package at "/work/app/packages/ui" whose package.json has name "ui" and declares the scripts:
      | name | command    |
      | story | storybook |
    And the nested package contains no lockfile
    And "/work/app/pnpm-lock.yaml" exists
    And the origin pane foreground cwd is "/work/app/packages/ui"
    When the sidebar opens
    Then the resolved project root is "/work/app/packages/ui"
    And the detected package manager is "npm"

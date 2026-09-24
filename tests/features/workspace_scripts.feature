@catalog @workspace
Feature: Declared workspace packages and their scripts
  A workspace snapshot exposes declared packages and preserves each script's package identity.

  Scenario Outline: Journal packages are accessible from the root and a member
    Given a disposable Journal-shaped pnpm workspace
    When the workspace sidebar opens from "<origin>"
    Then all six workspace groups and both empty packages are rendered
    And all workspace packages resolve to pnpm
    And the selected workspace script belongs to "<selected>"

    Examples:
      | origin   | selected |
      | .        | .        |
      | apps/mcp | apps/mcp |

  Scenario: Filtering a collapsed package keeps script identities and restores the tree
    Given a disposable Journal-shaped pnpm workspace
    When the workspace sidebar opens from "."
    And I filter workspace scripts by "dev"
    Then the three dev results are grouped under their package paths
    When I apply the workspace search
    Then no workspace script has been launched
    When I launch the selected workspace script
    Then exactly one background tab runs pnpm dev in "apps/auth"
    When I clear the workspace search
    Then the workspace member groups are collapsed again

  Scenario: A broken member is local to its group
    Given a disposable Journal-shaped pnpm workspace
    And the workspace member "apps/auth" has invalid JSON
    When the workspace sidebar opens from "apps/mcp"
    Then the broken member diagnostic and the mcp scripts are rendered
    When I launch the selected workspace script
    Then exactly one background tab runs pnpm dev in "apps/mcp"

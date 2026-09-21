# Use case: RunScript — every launch opens its own Herdr tab, in the background,
# inside the workspace captured with the catalogue. No state is tracked:
# relaunching means picking the script in the sidebar again.
# Assertions check the argument received by the package manager, not a quoting
# spelling of the command line.

@run
Feature: Run a script in a new background tab
  As a developer browsing the scripts of my package
  I want a script to start in its own tab without losing the sidebar
  So that I can fire several scripts in a row and read them when I want

  Background:
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command           |
      | dev   | vite              |
      | build | tsc && vite build |
      | test  | vitest run        |
    And the sidebar is open in the workspace "main"
    And the resolved project root is "/work/app"
    And the detected package manager is "npm"

  Scenario: Running the selected script with Enter
    Given the selection is on the script "dev"
    When I press "Enter"
    Then a new tab is created in the workspace "main" with focus false
    And the package manager "npm" is invoked in "/work/app"
    And the package manager receives a single script argument "dev"
    And the working directory of the new tab is "/work/app"

  Scenario Outline: A single left click anywhere on the row runs the script
    When I left-click the <zone> of the row of the script "test"
    Then a new tab is created in the workspace "main" with focus false
    And the package manager "npm" is invoked in "/work/app"
    And the package manager receives a single script argument "test"
    And the selection moves to the script "test"

    Examples: the whole row is the hit area, the play icon is only an affordance
      | zone           |
      | play icon      |
      | script name    |
      | command text   |
      | trailing space |

  Scenario: A single mouse down is enough, release does not launch again
    Given the selection is on the script "dev"
    When I left-click the row of the script "test"
    And I release the mouse on the row of the script "test"
    Then exactly 1 tab has been created in the workspace "main"
    And the package manager receives a single script argument "test"

  Scenario Outline: Clicking outside a script row runs nothing
    Given the selection is on the script "dev"
    When I left-click <target>
    Then no tab is created
    And the selection is still on the script "dev"

    Examples:
      | target                              |
      | the sidebar header                  |
      | the empty space below the list      |
      | the full command line at the bottom |

  Scenario: The new tab does not steal the focus
    When I run the script "dev"
    Then the new tab does not take the focus
    And the sidebar pane is still focused
    And the sidebar still lists 3 scripts
    And the selection is still on the script "dev"

  Scenario: Firing several scripts in a row
    When I run the script "dev"
    And I run the script "test"
    Then 2 tabs have been created in the workspace "main"
    And the sidebar pane is still focused

  Scenario: Every launch opens its own tab
    When I run the script "dev"
    And I run the script "dev"
    Then 2 tabs are running script "dev" with "npm"
    And no existing tab was reused

  Scenario: The new tab is labelled after the command
    When I run the script "build"
    Then the new tab is labelled with the invoked command for script "build"

  Scenario: Running with pnpm
    Given the detected package manager is "pnpm"
    When I run the script "build"
    Then the package manager "pnpm" is invoked in "/work/app"
    And the package manager receives a single script argument "build"

  Scenario: The script runs in the package directory, not in the plugin directory
    Given the origin pane foreground cwd was "/work/app/src/components" when the sidebar opened
    When I run the script "dev"
    Then the working directory of the new tab is "/work/app"

  Scenario: The script name reaches the shell as one literal argument
    Given the package declares a script named "test watch; $(id)"
    When I run the script "test watch; $(id)"
    Then the package manager receives a single script argument "test watch; $(id)"

  Scenario: The 40th selected script can be launched
    Given a project at "/work/app" whose package.json has name "app" and declares 40 scripts named s1 to s40
    And the sidebar is open on that catalogue
    And the 40th script is selected
    When I press "Enter"
    Then a new tab is created in the workspace "main" with focus false
    And the package manager receives a single script argument "s40"

  Scenario: A failed tab creation sends no input
    Given creating a tab will fail
    When I run the script "dev"
    Then no pane input is sent
    And no confirmed launch is announced
    And the catalogue is unchanged

  Scenario: A timeout after tab creation does not retry
    Given a tab is created with id "tab-dev"
    And sending input to its root pane times out
    When I run the script "dev"
    Then the sidebar shows the message "Script launch not confirmed" including tab id "tab-dev"
    And the launcher does not retry the send
    And the catalogue is still usable
    And the sidebar pane is still focused

  Scenario: A missing root pane after creation does not retry
    Given tab creation returns no usable root pane id
    When I run the script "dev"
    Then the sidebar shows the message "Script launch not confirmed"
    And no pane input is sent
    And the catalogue is still usable

  Scenario: A successful manual launch clears the previous launch error
    Given sending input to its root pane times out
    When I run the script "dev"
    Then the sidebar shows the message "Script launch not confirmed"
    When sending input succeeds again
    And I run the script "build"
    Then the message "Script launch not confirmed" is gone
    And the package manager receives a single script argument "build"

  Scenario: Clicking an error below a long catalogue does not launch a hidden row
    Given a project at "/work/app" whose package.json has name "app" and declares 40 scripts named s1 to s40
    And the sidebar is open on that catalogue
    And sending input to its root pane times out
    When I press "Enter"
    Then the sidebar shows the message "Script launch not confirmed"
    When I left-click the launch error message
    Then exactly 1 tab has been created in the workspace "main"

  Scenario: An empty command string is still forwarded
    Given the package declares a script named "noop" whose command is ""
    When I run the script "noop"
    Then the package manager "npm" receives a single script argument "noop"

  Scenario: A global focus change does not retarget the launch
    Given the selection is on the script "dev"
    And the global focus moves to another workspace while the launch is running
    When I run the script "dev"
    Then the new tab is created in the workspace "main"
    And the working directory of the new tab is "/work/app"

# Use case: ListScripts — runs once, when the sidebar TUI starts.
# The project root comes from the origin pane captured by toggle, then
# catalogue, manager and root stay frozen until the sidebar closes.
# Launching a script is out of this file; selection and run intent stay here.

@catalog
Feature: List the scripts of the current package.json
  As a developer with a sidebar open
  I want to see every script of the package I am working in
  So that I can pick one without reading the file

  Scenario: Listing every script of the package
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command           |
      | dev   | vite              |
      | build | tsc && vite build |
      | test  | vitest run        |
    And the origin pane foreground cwd is "/work/app"
    And the TestBackend is 32 columns wide and 24 rows tall
    When the sidebar opens
    Then the sidebar lists 3 scripts
    And the sidebar shows the script "build" with the command "tsc && vite build"
    And every script row shows a play icon
    And the sidebar header shows the package name "app" and the detected package manager
    And the selection is on the script "dev"

  Scenario: The scripts keep their declaration order
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name      | command     |
      | prebuild  | rimraf dist |
      | build     | tsc         |
      | postbuild | echo done   |
    And the origin pane foreground cwd is "/work/app"
    When the sidebar opens
    Then the sidebar lists the scripts in this order: prebuild, build, postbuild

  Scenario: Finding the package.json from a sub-directory
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command           |
      | dev   | vite              |
      | build | tsc && vite build |
      | test  | vitest run        |
    And the origin pane foreground cwd is "/work/app/src/components"
    When the sidebar opens
    Then the resolved project root is "/work/app"

  Scenario: The nearest package.json wins
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command           |
      | dev   | vite              |
      | build | tsc && vite build |
      | test  | vitest run        |
    And a nested package at "/work/app/packages/ui" whose package.json has name "ui" and declares the scripts:
      | name | command    |
      | story | storybook |
    And the origin pane foreground cwd is "/work/app/packages/ui/src"
    When the sidebar opens
    Then the resolved project root is "/work/app/packages/ui"
    And the sidebar lists the scripts in this order: story

  Scenario: An invalid nested package is not skipped for its parent
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name | command |
      | dev  | vite    |
    And a nested path "/work/app/packages/broken/package.json" that is not valid JSON
    And the origin pane foreground cwd is "/work/app/packages/broken/src"
    When the sidebar opens
    Then the sidebar shows the message "package.json is not valid JSON"
    And no script is listed
    And the resolved project root is not "/work/app"

  Scenario: An empty nested package is not skipped for its parent
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name | command |
      | dev  | vite    |
    And a nested package at "/work/app/packages/empty" whose package.json has name "empty" and has no "scripts" field
    And the origin pane foreground cwd is "/work/app/packages/empty"
    When the sidebar opens
    Then the sidebar shows the message "This package.json has no scripts"
    And the resolved project root is "/work/app/packages/empty"

  Scenario: The project root is frozen for the lifetime of the sidebar
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command           |
      | dev   | vite              |
      | build | tsc && vite build |
      | test  | vitest run        |
    And a project at "/work/other" whose package.json has name "other" and declares the scripts:
      | name  | command         |
      | start | node server.js  |
      | lint  | eslint .        |
    And the origin pane foreground cwd is "/work/app"
    And the sidebar is open on the project root "/work/app"
    When the origin pane foreground cwd becomes "/work/other"
    Then the resolved project root is still "/work/app"
    And the listed scripts are unchanged

  Scenario: The listed scripts stay frozen if package.json changes on disk
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command           |
      | dev   | vite              |
      | build | tsc && vite build |
      | test  | vitest run        |
    And the origin pane foreground cwd is "/work/app"
    And the sidebar is open on the project root "/work/app"
    When the package.json at "/work/app" is rewritten with only the script "lint"
    Then the sidebar still lists the scripts in this order: dev, build, test

  Scenario: The project root is resolved again at the next opening
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command           |
      | dev   | vite              |
      | build | tsc && vite build |
      | test  | vitest run        |
    And a project at "/work/other" whose package.json has name "other" and declares the scripts:
      | name  | command         |
      | start | node server.js  |
      | lint  | eslint .        |
    And the sidebar was opened on the project root "/work/app" and then closed with "q"
    And the origin pane foreground cwd is now "/work/other"
    When I invoke the "herdr-npm.toggle" action
    Then the resolved project root is "/work/other"
    And the sidebar lists the scripts in this order: start, lint

  Scenario: The live foreground cwd is preferred over the start directory
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command           |
      | dev   | vite              |
      | build | tsc && vite build |
      | test  | vitest run        |
    And a project at "/work/other" whose package.json has name "other" and declares the scripts:
      | name  | command         |
      | start | node server.js  |
      | lint  | eslint .        |
    And the origin pane start cwd is "/work/other"
    And the origin pane foreground cwd is "/work/app"
    When the sidebar opens
    Then the resolved project root is "/work/app"

  Scenario: The start directory is used when the live cwd is missing
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command           |
      | dev   | vite              |
      | build | tsc && vite build |
      | test  | vitest run        |
    And the origin pane start cwd is "/work/app"
    And the origin pane has no foreground cwd
    When the sidebar opens
    Then the resolved project root is "/work/app"
    And the sidebar shows the message "Current directory unavailable; using pane start directory"

  Scenario: Neither live cwd nor start directory can be used
    Given the origin pane has no foreground cwd
    And the origin pane has no start cwd
    When the sidebar opens
    Then the sidebar shows the message "Cannot determine project directory"
    And no script is listed
    And no script can be launched

  Scenario: No package.json anywhere above the origin pane
    Given the origin pane foreground cwd is "/tmp/scratch"
    And no package.json exists in "/tmp/scratch" nor in any of its parents
    When the sidebar opens
    Then the sidebar shows the message "No package.json found"
    And no script is listed
    And no script can be launched
    And the herdr-npm process is still running

  Scenario: An invalid package.json does not crash the sidebar
    Given a project at "/work/app" whose package.json is not valid JSON
    And the origin pane foreground cwd is "/work/app"
    When the sidebar opens
    Then the sidebar shows the message "package.json is not valid JSON"
    And no script is listed
    And the herdr-npm process is still running
    And no script can be launched

  Scenario: A package.json whose root is not an object
    Given a project at "/work/app" whose package.json root is a JSON array
    And the origin pane foreground cwd is "/work/app"
    When the sidebar opens
    Then the sidebar shows the message "package.json is not valid JSON"
    And no script is listed
    And no script can be launched

  Scenario: A package.json that cannot be read
    Given a project at "/work/app" whose package.json cannot be read
    And the origin pane foreground cwd is "/work/app"
    When the sidebar opens
    Then the sidebar shows the message "Cannot read package.json"
    And no script is listed
    And no script can be launched
    And the herdr-npm process is still running

  Scenario: A package.json without a scripts field
    Given a project at "/work/app" whose package.json has name "app" and has no "scripts" field
    And the origin pane foreground cwd is "/work/app"
    When the sidebar opens
    Then the sidebar shows the message "This package.json has no scripts"
    And no script is listed
    And no script can be launched

  Scenario: An empty scripts field
    Given a project at "/work/app" whose package.json has name "app" and has an empty "scripts" field
    And the origin pane foreground cwd is "/work/app"
    When the sidebar opens
    Then the sidebar shows the message "This package.json has no scripts"
    And no script is listed
    And no script can be launched

  Scenario Outline: scripts must be an object of strings
    Given a project at "/work/app" whose package.json has name "app" and whose "scripts" field is <shape>
    And the origin pane foreground cwd is "/work/app"
    When the sidebar opens
    Then the sidebar shows the message "package.json scripts must be an object of strings"
    And no script is listed
    And no script can be launched

    Examples:
      | shape                                 |
      | an array                              |
      | a string                              |
      | an object with a non-string value     |

  Scenario: An empty command string is still listed
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name | command |
      | noop |         |
      | dev  | vite    |
    And the origin pane foreground cwd is "/work/app"
    When the sidebar opens
    Then the sidebar lists the scripts in this order: noop, dev
    And the sidebar shows the script "noop" with the command ""

  Scenario: A missing package name uses the directory name
    Given a project at "/work/app" whose package.json has no name and declares the scripts:
      | name | command |
      | dev  | vite    |
    And the origin pane foreground cwd is "/work/app"
    When the sidebar opens
    Then the sidebar header shows the package name "app" and the detected package manager

  Scenario: An empty package name uses the directory name
    Given a project at "/work/app" whose package.json has name "" and declares the scripts:
      | name | command |
      | dev  | vite    |
    And the origin pane foreground cwd is "/work/app"
    When the sidebar opens
    Then the sidebar header shows the package name "app" and the detected package manager

  Scenario: A non-string package name uses the directory name
    Given a project at "/work/app" whose package.json has a non-string name and declares the scripts:
      | name | command |
      | dev  | vite    |
    And the origin pane foreground cwd is "/work/app"
    When the sidebar opens
    Then the sidebar header shows the package name "app" and the detected package manager

  Scenario: Navigating the script list stays inside the bounds
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command           |
      | dev   | vite              |
      | build | tsc && vite build |
      | test  | vitest run        |
    And the origin pane foreground cwd is "/work/app"
    And the sidebar is open with the selection on the script "dev"
    When I press "j"
    Then the selection is on the script "build"
    When I press "k"
    Then the selection is on the script "dev"
    When I press the down arrow
    Then the selection is on the script "build"
    When I press the up arrow
    Then the selection is on the script "dev"
    When I press "k"
    Then the selection is still on the script "dev"
    When I move the selection to the script "test"
    And I press "j"
    Then the selection is still on the script "test"

  Scenario: A long list scrolls instead of hiding scripts
    Given a project at "/work/app" whose package.json has name "app" and declares 40 scripts named s1 to s40
    And the origin pane foreground cwd is "/work/app"
    And the TestBackend is 32 columns wide and 24 rows tall
    When the sidebar opens
    Then the sidebar lists 40 scripts
    And the selection is on the first script
    When I move the selection to the 40th script
    Then the list scrolls to keep the selection visible
    And the 40th script is selected
    And no run intent has been emitted

  @v1_1_wheel
  Scenario: The mouse wheel scrolls a long list without changing the selection
    Given a project at "/work/app" whose package.json has name "app" and declares 40 scripts named s1 to s40
    And the origin pane foreground cwd is "/work/app"
    And the TestBackend is 32 columns wide and 24 rows tall
    When the sidebar opens
    And I roll the mouse wheel down on the list
    Then the list offset increases by 3
    And the selection is on the first script
    And no run intent has been emitted
    When I redraw the sidebar twice at the same size
    Then the list offset is unchanged
    When the TestBackend is resized to 40 columns and 20 rows
    Then the list offset is at most the last full screen
    When I roll the mouse wheel up on the list
    Then the list offset is 0

  @v1_1_wheel
  Scenario: The mouse wheel stops at both ends of a long list
    Given a project at "/work/app" whose package.json has name "app" and declares 40 scripts named s1 to s40
    And the origin pane foreground cwd is "/work/app"
    And the TestBackend is 32 columns wide and 24 rows tall
    When the sidebar opens
    And I roll the mouse wheel up on the list
    Then the list offset is 0
    When I roll the mouse wheel down until the last page
    Then the list offset is at the last full screen
    When I roll the mouse wheel down on the list
    Then the list offset is at the last full screen

  @v1_1_wheel
  Scenario: A short list ignores the mouse wheel
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command           |
      | dev   | vite              |
      | build | tsc && vite build |
      | test  | vitest run        |
    And the origin pane foreground cwd is "/work/app"
    And the TestBackend is 32 columns wide and 24 rows tall
    When the sidebar opens
    And I roll the mouse wheel down on the list
    Then the list offset is 0
    And the selection is still on the first script
    And no run intent has been emitted

  @v1_1_wheel
  Scenario: A click after scrolling hits the visible row
    Given a project at "/work/app" whose package.json has name "app" and declares 40 scripts named s1 to s40
    And the origin pane foreground cwd is "/work/app"
    And the TestBackend is 32 columns wide and 24 rows tall
    When the sidebar opens
    And I roll the mouse wheel down on the list
    And I left-click the play icon of the first visible row
    Then the first visible script is selected
    And a single run intent is emitted for the first visible script

  @v1_1_wheel
  Scenario: Keyboard navigation at the bound brings the selection back into view
    Given a project at "/work/app" whose package.json has name "app" and declares 40 scripts named s1 to s40
    And the origin pane foreground cwd is "/work/app"
    And the TestBackend is 32 columns wide and 24 rows tall
    When the sidebar opens
    And I roll the mouse wheel down until the selection is off-screen
    Then the selected script is not in the visible window
    When I press "k"
    Then the selection is still on the first script
    And the list scrolls to keep the selection visible
    And no run intent has been emitted

  @v1_1_wheel
  Scenario: The mouse wheel does nothing on the header
    Given a project at "/work/app" whose package.json has name "app" and declares 40 scripts named s1 to s40
    And the origin pane foreground cwd is "/work/app"
    And the TestBackend is 32 columns wide and 24 rows tall
    When the sidebar opens
    And I roll the mouse wheel down on the sidebar header
    Then the list offset is 0
    And the selection is still on the first script
    And no run intent has been emitted

  @v1_1_wheel
  Scenario: The mouse wheel does nothing when the terminal is too small
    Given a project at "/work/app" whose package.json has name "app" and declares 40 scripts named s1 to s40
    And the origin pane foreground cwd is "/work/app"
    And the TestBackend interior is 11 columns by 3 rows
    When the sidebar opens
    Then the sidebar shows the message "Terminal too small"
    When I roll the mouse wheel down on the pane
    Then the sidebar shows the message "Terminal too small"
    And no script can be launched

  Scenario Outline: Text too long for the column is cut with an ellipsis
    Given the TestBackend is 32 columns wide and 24 rows tall
    And a project at "/work/app" whose package.json has name "app" and declares a script whose <field> is longer than the column
    And the origin pane foreground cwd is "/work/app"
    When the sidebar opens
    Then that <field> is cut with an ellipsis
    And the row still shows its play icon

    Examples:
      | field   |
      | name    |
      | command |

  Scenario: Wide characters use terminal display width
    Given the TestBackend is 32 columns wide and 24 rows tall
    And a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name | command        |
      | 日本語 | echo 日本語     |
    And the origin pane foreground cwd is "/work/app"
    When the sidebar opens
    Then the script name and command are laid out using terminal cell width
    And overflowing text is cut with an ellipsis without splitting a wide character

  Scenario: The selected script shows its full command on one footer line
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command                                           |
      | build | tsc && vite build --config a-very-long-file-name |
    And the origin pane foreground cwd is "/work/app"
    And the TestBackend is 32 columns wide and 24 rows tall
    When the sidebar opens
    And the selection moves to the script "build"
    Then the footer shows the command of "build" on a single line
    And the footer shows how to scroll that line with h and l

  Scenario: h and l scroll the footer command without changing the selection
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command                                           |
      | build | tsc && vite build --config a-very-long-file-name |
    And the origin pane foreground cwd is "/work/app"
    And the TestBackend is 32 columns wide and 24 rows tall
    And the sidebar is open with the selection on the script "build"
    When I press "l"
    Then the footer command scrolls right by one cell
    And the selection is still on the script "build"
    When I press "h"
    Then the footer command scrolls left by one cell
    And the selection is still on the script "build"

  Scenario: Changing the selection resets the footer scroll
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command                                           |
      | dev   | vite                                              |
      | build | tsc && vite build --config a-very-long-file-name |
    And the origin pane foreground cwd is "/work/app"
    And the TestBackend is 32 columns wide and 24 rows tall
    And the sidebar is open with the selection on the script "build"
    And the footer command has been scrolled to the right
    When I press "k"
    Then the selection is on the script "dev"
    And the footer scroll offset is 0

  Scenario: A terminal that is too small shows a blocking message
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command           |
      | dev   | vite              |
      | build | tsc && vite build |
      | test  | vitest run        |
    And the origin pane foreground cwd is "/work/app"
    And the TestBackend interior is 11 columns by 3 rows
    When the sidebar opens
    Then the sidebar shows the message "Terminal too small"
    And only q and the toggle action remain usable
    And no script can be launched

  Scenario: Enlarging a too-small terminal restores the catalogue
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command           |
      | dev   | vite              |
      | build | tsc && vite build |
      | test  | vitest run        |
    And the origin pane foreground cwd is "/work/app"
    And the sidebar is showing "Terminal too small"
    When the TestBackend interior grows to 32 columns by 24 rows
    Then the sidebar lists 3 scripts
    And the message "Terminal too small" is gone

  @v1_1_icon
  Scenario: Mouse coordinates follow scrolling and resizing
    Given a project at "/work/app" whose package.json has name "app" and declares 40 scripts named s1 to s40
    And the origin pane foreground cwd is "/work/app"
    And the TestBackend is 32 columns wide and 24 rows tall
    And the sidebar is open with the 40th script selected and scrolled into view
    When I left-click the visible row of the 40th script
    Then the 40th script is selected
    And a single run intent is emitted for script "s40"
    When the TestBackend is resized to 40 columns and 20 rows
    And I left-click the visible row of the 40th script
    Then a single run intent is emitted for script "s40"

  @v1_1_icon
  Scenario Outline: A left click on the play icon selects it and emits a run intent
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command           |
      | dev   | vite              |
      | build | tsc && vite build |
      | test  | vitest run        |
    And the origin pane foreground cwd is "/work/app"
    And the TestBackend is 32 columns wide and 24 rows tall
    And the sidebar is open with the selection on the script "dev"
    When I left-click the <zone> of the row of the script "test"
    Then the selection is on the script "test"
    And a single run intent is emitted for script "test"

    Examples:
      | zone      |
      | play icon |

  @v1_1_icon
  Scenario Outline: A left click elsewhere on a script row selects it without launching
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command           |
      | dev   | vite              |
      | build | tsc && vite build |
      | test  | vitest run        |
    And the origin pane foreground cwd is "/work/app"
    And the TestBackend is 32 columns wide and 24 rows tall
    And the sidebar is open with the selection on the script "dev"
    When I left-click the <zone> of the row of the script "test"
    Then the selection is on the script "test"
    And no run intent has been emitted

    Examples:
      | zone           |
      | script name    |
      | command text   |
      | trailing space |

  @v1_1_icon
  Scenario: The play-icon gutter stops before the script name
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command           |
      | dev   | vite              |
      | build | tsc && vite build |
      | test  | vitest run        |
    And the origin pane foreground cwd is "/work/app"
    And the TestBackend is 32 columns wide and 24 rows tall
    And the sidebar is open with the selection on the script "dev"
    When I left-click column 3 of the row of the script "test"
    Then the selection is on the script "test"
    And no run intent has been emitted
    When I left-click column 2 of the row of the script "test"
    Then a single run intent is emitted for script "test"

  Scenario: Mouse release and movement do not emit a run intent
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command           |
      | dev   | vite              |
      | build | tsc && vite build |
      | test  | vitest run        |
    And the origin pane foreground cwd is "/work/app"
    And the sidebar is open with the selection on the script "dev"
    When I release the mouse on the row of the script "test"
    And I move the mouse across the row of the script "test"
    Then no run intent has been emitted
    And the selection is still on the script "dev"

  Scenario Outline: Clicking outside a script row does not select or launch
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name  | command           |
      | dev   | vite              |
      | build | tsc && vite build |
      | test  | vitest run        |
    And the origin pane foreground cwd is "/work/app"
    And the sidebar is open with the selection on the script "dev"
    When I left-click <target>
    Then no run intent has been emitted
    And the selection is still on the script "dev"

    Examples:
      | target                              |
      | the sidebar header                  |
      | the empty space below the list      |
      | the full command line at the bottom |

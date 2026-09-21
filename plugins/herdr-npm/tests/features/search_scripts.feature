# Use case: SearchScripts — filter the frozen catalogue by name.
# The package.json is never reread. Launch still goes through RunIntent.

@search @v1_1_search
Feature: Search scripts by fuzzy name
  As a developer with forty scripts
  I want to type a few letters and land on the right one
  So that I do not have to scroll the whole catalogue

  Background:
    Given a project at "/work/app" whose package.json has name "app" and declares the scripts:
      | name     | command           |
      | dev      | vite              |
      | build    | tsc && vite build |
      | test     | vitest run        |
      | lint     | eslint .          |
      | start    | node server.js    |
    And the origin pane foreground cwd is "/work/app"
    And the TestBackend is 32 columns wide and 24 rows tall
    And the sidebar is open with the selection on the script "dev"

  Scenario: Slash opens the search field
    When I press "/"
    Then the search field is open
    And the search query is ""

  Scenario: Ctrl+F opens the search field
    When I press Ctrl+F
    Then the search field is open

  Scenario: A click on the magnifier opens the search field
    When I left-click the search magnifier
    Then the search field is open

  Scenario: Letters including q j k are typed into the field
    When I press "/"
    And I type "qjk"
    Then the search query is "qjk"
    And the herdr-npm process is still running

  Scenario: Typing filters the list immediately
    When I press "/"
    And I type "bu"
    Then the sidebar shows the matching scripts: build
    And the selection is on the script "build"
    And no run intent has been emitted

  Scenario: Enter applies the filter without launching and selects the first result
    When I press "/"
    And I type "t"
    And I press the down arrow
    And I press "Enter"
    Then the search field is closed
    And the search query is "t"
    And the selection is on the script "test"
    And no run intent has been emitted

  Scenario: Enter on an applied filter launches the selected script
    When I press "/"
    And I type "bu"
    And I press "Enter"
    And I press "Enter"
    Then a single run intent is emitted for script "build"

  Scenario: The play icon launches the filtered script
    When I press "/"
    And I type "bu"
    And I press "Enter"
    And I left-click the play icon of the row of the script "build"
    Then a single run intent is emitted for script "build"

  Scenario: Esc from the field restores the full list
    When I press "/"
    And I type "bu"
    And I press "Esc"
    Then the search field is closed
    And the search query is ""
    And the sidebar lists 5 scripts
    And the herdr-npm process is still running

  Scenario: Esc on an applied filter clears it before closing the pane
    When I press "/"
    And I type "bu"
    And I press "Enter"
    And I press "Esc"
    Then the search field is closed
    And the search query is ""
    And the sidebar lists 5 scripts
    And the herdr-npm process is still running

  Scenario: No match shows a message and nothing is launchable
    When I press "/"
    And I type "xyz"
    Then no script matches "xyz"
    And no script can be launched

  Scenario: Backspace restores previous results
    When I press "/"
    And I type "buz"
    Then no script matches "buz"
    When I press "Backspace"
    Then the sidebar shows the matching scripts: build

  Scenario: The mouse wheel uses the filtered length and survives a redraw
    Given a project at "/work/app" whose package.json has name "app" and declares 40 scripts named s1 to s40
    And the origin pane foreground cwd is "/work/app"
    And the TestBackend is 32 columns wide and 24 rows tall
    When the sidebar opens
    And I press "/"
    And I type "s"
    And I roll the mouse wheel down on the list
    Then the list offset increases by 3
    When I redraw the sidebar twice at the same size
    Then the list offset is unchanged
    And no run intent has been emitted

  Scenario: Search is unavailable when the terminal is too small
    Given the TestBackend interior is 11 columns by 3 rows
    When the sidebar opens
    Then the sidebar shows the message "Terminal too small"
    When I press "/"
    Then the search field is closed

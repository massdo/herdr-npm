# Use case: ToggleSidebar — bound to herdr-npm.toggle.
# Two states only, decided from pane.list of the captured origin tab:
#   no pane with token herdr_npm_sidebar=v1 in that tab -> OPEN and focus it
#   one such pane                                        -> CLOSE it, wherever focus is in the tab
# Recognition never uses the visible label "npm" alone.
# Unreadable snapshot, missing origin, missing working target or several
# recognised sidebars -> error and no mutation.
# This file does not read package.json.

@toggle
Feature: Toggle the herdr-npm sidebar
  As a developer working in a Herdr tab
  I want one shortcut that opens the npm scripts sidebar, and closes it
  So that I never have to open package.json to recall a script name

  Background:
    Given Herdr is running with the "herdr-npm" plugin installed

  Scenario: Opening the sidebar in a tab that has none
    Given the focused tab has no pane carrying token "herdr_npm_sidebar" equal to "v1"
    And the origin action context is workspace "main", tab "one", pane "editor"
    And pane "editor" is the leftmost working pane of that tab
    When I invoke the "herdr-npm.toggle" action
    Then a sidebar pane is opened to the left of pane "editor"
    And the sidebar pane is labelled "npm"
    And the sidebar pane carries token "herdr_npm_sidebar" equal to "v1"
    And the sidebar pane is focused
    And the preferred outer width of the sidebar is 32 columns
    And the sidebar height matches the height of pane "editor"

  Scenario: Closing the sidebar from the sidebar itself
    Given the focused tab already has a herdr-npm sidebar recognised by token
    And that sidebar was split from pane "editor"
    And the focus is on the sidebar pane
    When I invoke the "herdr-npm.toggle" action
    Then the recognised sidebar pane is closed
    And the focus returns to pane "editor"

  Scenario: Closing the sidebar from another pane of the tab
    Given the focused tab already has a herdr-npm sidebar recognised by token
    And the focus is on another pane of that tab
    When I invoke the "herdr-npm.toggle" action
    Then the recognised sidebar pane is closed
    And the focus does not move

  Scenario: Two toggles open then close
    Given the focused tab has no pane carrying token "herdr_npm_sidebar" equal to "v1"
    When I invoke the "herdr-npm.toggle" action
    Then a sidebar pane recognised by token is open in the focused tab
    When I invoke the "herdr-npm.toggle" action
    Then that sidebar pane is closed
    And the focused tab has no pane carrying token "herdr_npm_sidebar" equal to "v1"

  Scenario: The sidebar of another tab is never touched
    Given the tab "one" has a herdr-npm sidebar recognised by token
    And the focused tab is "two" and has no pane carrying token "herdr_npm_sidebar" equal to "v1"
    When I invoke the "herdr-npm.toggle" action
    Then a sidebar pane is opened in the tab "two"
    And the sidebar pane of the tab "one" is left untouched

  Scenario: A pane labelled npm without the token is not the sidebar
    Given the focused tab has a pane labelled "npm" that does not carry token "herdr_npm_sidebar" equal to "v1"
    When I invoke the "herdr-npm.toggle" action
    Then a sidebar pane recognised by token is opened in the focused tab
    And the foreign pane labelled "npm" is left untouched

  Scenario: Concurrent toggles are serialised to open then close
    Given the focused tab has no pane carrying token "herdr_npm_sidebar" equal to "v1"
    When two "herdr-npm.toggle" invocations start at the same time
    Then the invocations run one after the other under the launcher lock
    And they open a sidebar then close it

  Scenario: An unreadable pane list is an error without mutation
    Given the pane list returned by Herdr cannot be interpreted
    When I invoke the "herdr-npm.toggle" action
    Then the launcher exits with a non-zero status
    And no pane is opened
    And no existing pane is closed by the plugin

  Scenario: A missing origin context is an error without mutation
    Given the origin workspace, tab or pane of the action is missing
    When I invoke the "herdr-npm.toggle" action
    Then the launcher exits with a non-zero status
    And no pane is opened
    And no existing pane is closed by the plugin

  Scenario: A changed origin context is an error without mutation
    Given the captured origin pane is no longer present in the pane list
    When I invoke the "herdr-npm.toggle" action
    Then the launcher exits with a non-zero status
    And no pane is opened
    And no existing pane is closed by the plugin

  Scenario: Several recognised sidebars are an error without mutation
    Given the focused tab has two panes carrying token "herdr_npm_sidebar" equal to "v1"
    When I invoke the "herdr-npm.toggle" action
    Then the launcher exits with a non-zero status
    And neither recognised pane is closed
    And no additional pane is opened

  Scenario: No working-pane target is an error without mutation
    Given the focused tab has no working pane that can be used as a split target
    When I invoke the "herdr-npm.toggle" action
    Then the launcher exits with a non-zero status
    And no pane is opened

  Scenario: An uncertain transport error is never retried automatically
    Given opening a pane returns an uncertain transport error
    When I invoke the "herdr-npm.toggle" action
    Then the launcher does not retry the opening
    And the launcher reports that the layout must be inspected before another attempt
    And the launcher does not announce a confirmed open or close

  Scenario: The origin context is captured before any I/O
    Given the origin action context is workspace "main", tab "one", pane "editor"
    And the global focus moves to another tab while the toggle is running
    When I invoke the "herdr-npm.toggle" action
    Then the sidebar is opened in tab "one"
    And no pane of the newly focused tab is mutated

  Scenario: A vertically split layout keeps the target pane height
    Given the working pane "editor" already occupies only the top half of the focused tab
    And the focused tab has no pane carrying token "herdr_npm_sidebar" equal to "v1"
    When I invoke the "herdr-npm.toggle" action
    Then the sidebar height matches the height of pane "editor"
    And the sidebar is not stretched to the full tab height
    And the other splits of the tab keep their size

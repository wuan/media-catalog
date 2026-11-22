# Spec Delta

## ADDED Requirements

### Requirement: Protect build state during events-only runs

An events-only run SHALL NOT delete unseen state rows, reset item thumbnail status, prune orphan thumbnails, or trigger the mount-drop guard, because no scan observes the media tree. It SHALL still acquire the build lock before republishing output.

#### Scenario: State rows preserved
- **WHEN** an events-only run completes
- **THEN** every item row remains in the state database with its previous thumbnail status, metadata, and last-seen run

#### Scenario: Prune and cleanup suppressed
- **WHEN** the generator runs with `--events-only --prune`
- **THEN** no thumbnails are pruned and no state rows are deleted

#### Scenario: Lock still enforced
- **WHEN** an events-only run is in progress and a second build starts against the same state root
- **THEN** the second build exits with a clear message and does not modify output

#### Scenario: Mount-drop guard inactive
- **WHEN** the media root is unmounted and an events-only run executes
- **THEN** the run is not blocked and reports no mount-drop warning

#### Scenario: Next full run cleans up normally
- **WHEN** a full build runs after an events-only run
- **THEN** deletion detection and pruning behave exactly as if the events-only run had not happened

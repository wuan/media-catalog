# Spec Delta

## ADDED Requirements

### Requirement: Report probe failures by cause

The generator SHALL report probe failures separately from other errors in the run summary, with the number of failed and attempted probes. It SHALL group failures by their root cause and show, for each cause, its count and a bounded number of sample paths, ordered by count from highest to lowest.

#### Scenario: Probe failure line
- **WHEN** a run attempts to probe 10 items and 3 probes fail
- **THEN** the summary contains `Probe failures: 3 of 10`, separate from the thumbnail counts and the overall error count

#### Scenario: Failures grouped by cause
- **WHEN** 500 probes fail because the files cannot be opened due to permissions and 2 probes fail because the files are corrupt
- **THEN** the summary lists the permission cause first with its count of 500 and a few sample paths, followed by the corrupt-file cause with its count of 2, instead of only listing individual files

#### Scenario: No probe failures
- **WHEN** every attempted probe succeeds
- **THEN** the summary reports `Probe failures: 0 of <attempted>` and lists no failure causes

#### Scenario: Nothing to probe
- **WHEN** a run has no items that need probing
- **THEN** the summary reports `Probe failures: 0 of 0` and prints no systemic probe warning

### Requirement: Warn on systemic probe failure

The generator SHALL print a prominent warning when at least 4 probes were attempted and at least half of them failed. The warning SHALL say that the originals are probably unreadable in this environment and that the published catalog will lack dimensions and camera metadata for those items.

#### Scenario: Every probe fails
- **WHEN** a run attempts to probe 72,946 items and all of them fail with the same cause
- **THEN** the summary prints the systemic probe warning naming the failed and attempted counts, and the run exits non-zero

#### Scenario: Isolated failures
- **WHEN** a run attempts to probe 100 items and 2 fail
- **THEN** the failures are reported by cause but no systemic probe warning is printed

#### Scenario: Too few probes to judge
- **WHEN** a run attempts to probe 3 items and all of them fail
- **THEN** the failures are reported by cause but no systemic probe warning is printed

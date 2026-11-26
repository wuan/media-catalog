# Spec Delta

## MODIFIED Requirements

### Requirement: Order catalog items deterministically
The generator SHALL order years newest first, months within each year by month number ascending, and items within each month by capture timestamp ascending (oldest first), with the relative path as a stable tie-break, so repeated runs produce the same order.

#### Scenario: Stable ordering across runs
- **WHEN** the same unchanged media tree is scanned twice
- **THEN** the emitted pages and JSON contain items in the same order both times

#### Scenario: Same capture timestamp
- **WHEN** two items share a capture timestamp
- **THEN** their relative paths break the tie deterministically

#### Scenario: Years are newest first
- **WHEN** the catalog contains items from 2023 and 2024
- **THEN** the root index and each year's position in the published index list 2024 before 2023

#### Scenario: Months ascend within a year
- **WHEN** a year contains items in January, May, and December
- **THEN** its year page and published months array list January before May before December

#### Scenario: Items are oldest first within a month
- **WHEN** a month contains items captured on 2024-05-01, 2024-05-02, and 2024-05-03
- **THEN** the month page and shard list the 2024-05-01 item before the later items

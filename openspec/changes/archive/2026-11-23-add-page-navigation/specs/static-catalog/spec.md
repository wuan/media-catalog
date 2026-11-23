# Spec Delta

## ADDED Requirements

### Requirement: Navigate between adjacent months

The runtime SHALL provide previous and next month links on each month page. Each link SHALL name its destination month and year and SHALL target the nearest month page that exists in the published catalog in chronological order, crossing year boundaries and skipping months without items. The runtime SHALL omit a direction when no such month page exists, so navigation never links to a missing page. The links SHALL be plain anchors that work without JavaScript.

#### Scenario: Adjacent months in the same year
- **WHEN** a visitor opens `/2024/05/` and the catalog contains `/2024/04/` and `/2024/06/`
- **THEN** the page links to April 2024 as the previous month and June 2024 as the next month

#### Scenario: Missing months are skipped
- **WHEN** `/2024/05/` exists, `/2024/04/` and `/2024/06/` do not, and `/2024/03/` exists
- **THEN** the previous link points to March 2024 instead of a missing April page

#### Scenario: Cross-year navigation
- **WHEN** a visitor opens `/2024/01/` and the nearest earlier month page is `/2023/12/`
- **THEN** the previous link points to December 2023

#### Scenario: Earliest month page
- **WHEN** a month page is the earliest month page in the catalog
- **THEN** the page renders no previous month link

#### Scenario: Latest month page
- **WHEN** a month page is the latest month page in the catalog
- **THEN** the page renders no next month link

#### Scenario: Single month page
- **WHEN** the catalog contains exactly one month page
- **THEN** the page renders neither a previous nor a next month link

#### Scenario: Navigation repeated at the bottom
- **WHEN** a month page renders
- **THEN** the top and bottom navigation positions expose the same previous and next month links

#### Scenario: JavaScript disabled
- **WHEN** the browser has JavaScript disabled
- **THEN** the previous and next month links still navigate as plain anchors

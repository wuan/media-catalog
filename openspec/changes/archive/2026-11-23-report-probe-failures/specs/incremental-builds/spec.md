# Spec Delta

## ADDED Requirements

### Requirement: Retry failed probes on later runs

The generator SHALL probe an unchanged decodable image or video again on the next run when its previous probe failed, and SHALL keep doing so on each run until a probe succeeds. Once a probe succeeds, later runs with an unchanged fingerprint SHALL reuse the stored metadata without reopening the source.

#### Scenario: Recovery after the environment is fixed
- **WHEN** a JPEG's probe fails because the file cannot be read, and a later run with the same file size and modification time can read it
- **THEN** the later run probes it again and the published catalog records its upright width, height, and camera metadata

#### Scenario: Still unreadable
- **WHEN** a decodable item's probe failed on the previous run and the file is still unreadable
- **THEN** the run attempts the probe again, counts it as a probe failure, and keeps the item cataloged with its fallback capture time

#### Scenario: No retry after success
- **WHEN** a previously failed item was successfully probed and its fingerprint is unchanged
- **THEN** the next run reuses the stored metadata and does not reopen the source

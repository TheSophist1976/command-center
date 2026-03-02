### Requirement: Daily task file backup
The system SHALL create a backup of the task file each time the application starts. The backup SHALL be stored in a `.backups/` directory adjacent to the task file, named `tasks-YYYY-MM-DD.md` using the current local date. If a backup for today already exists, it SHALL be overwritten. If the task file does not exist, no backup SHALL be created.

#### Scenario: Backup created on startup
- **WHEN** the application starts and the task file exists
- **THEN** the system SHALL copy the task file to `.backups/tasks-YYYY-MM-DD.md` in the same parent directory

#### Scenario: Backup directory created if missing
- **WHEN** the `.backups/` directory does not exist
- **THEN** the system SHALL create it before writing the backup file

#### Scenario: No task file means no backup
- **WHEN** the application starts and the task file does not exist
- **THEN** no backup SHALL be created and no error SHALL be reported

#### Scenario: Backup overwrites same-day file
- **WHEN** the application starts and a backup for today already exists
- **THEN** the existing backup SHALL be overwritten with the current task file contents

### Requirement: Backup retention policy
The system SHALL retain at most 7 backup files. After writing a backup, the system SHALL delete any excess backup files, keeping only the 7 most recent (by filename date sort). Backup files are identified by the pattern `tasks-*.md` in the `.backups/` directory.

#### Scenario: Prune old backups
- **WHEN** a backup is written and 10 backup files exist in `.backups/`
- **THEN** the 3 oldest files (by filename) SHALL be deleted, leaving 7

#### Scenario: Fewer than 7 backups
- **WHEN** a backup is written and only 3 backup files exist
- **THEN** no files SHALL be deleted

### Requirement: Backup is best-effort
Backup operations SHALL NOT cause the application to fail or display errors. If any backup operation fails (directory creation, file copy, pruning), the failure SHALL be silently ignored and the application SHALL continue normally.

#### Scenario: Backup directory creation fails
- **WHEN** the `.backups/` directory cannot be created (e.g., permissions)
- **THEN** the application SHALL continue without creating a backup

#### Scenario: File copy fails
- **WHEN** the task file cannot be copied to the backup location
- **THEN** the application SHALL continue without reporting an error

### Requirement: Backups are not used by the application
Backup files SHALL NOT be read, loaded, or referenced by any application functionality. They exist solely for manual user recovery.

#### Scenario: Application ignores backup directory
- **WHEN** the application loads tasks or performs any operation
- **THEN** it SHALL NOT read from or reference the `.backups/` directory

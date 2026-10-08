## ADDED Requirements

### Requirement: `remote-host` config key
The config file MAY contain a `remote-host` key naming one hostname (no scheme or port, e.g. `desk.tail1234.ts.net`) that `task serve` SHALL accept requests for in addition to `localhost` and `127.0.0.1`. The value SHALL be read when `task serve` starts; changing it requires a restart.

#### Scenario: Remote host configured
- **WHEN** config contains `remote-host: desk.tail1234.ts.net` and `task serve` starts
- **THEN** the server SHALL accept requests with `Host: desk.tail1234.ts.net` subject to sign-in, and print the remote URL in its startup banner

#### Scenario: Remote host unset
- **WHEN** config has no `remote-host`
- **THEN** the server SHALL behave as before: only local requests are accepted

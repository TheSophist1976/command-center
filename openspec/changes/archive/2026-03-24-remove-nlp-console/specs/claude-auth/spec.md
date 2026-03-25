## REMOVED Requirements

### Requirement: Claude API key storage
**Reason**: The Claude API key is no longer used. The NLP console (the only consumer) is removed.
**Migration**: No replacement. Existing `claude_api_key` files are inert and can be deleted manually.

### Requirement: Environment variable fallback
**Reason**: `ANTHROPIC_API_KEY` is no longer read by the application.
**Migration**: No replacement.

### Requirement: Auth claude CLI subcommand
**Reason**: `task auth claude` is removed. The Claude API key is no longer managed by this tool.
**Migration**: No replacement.

### Requirement: Auth status includes Claude key
**Reason**: Claude key status is removed from `task auth status` output.
**Migration**: `task auth status` will only report Todoist and Slack token status.

### Requirement: Auth revoke includes Claude key
**Reason**: `task auth revoke` no longer deletes the Claude API key file.
**Migration**: Delete the file manually at `{config_dir}/task-manager/claude_api_key` if desired.

# Schedule

## Environment isolation

Scheduled tasks run with a scrubbed environment. The default allowlist:

- **Exact:** PATH, HOME, USER, LOGNAME, SHELL, LANG, TZ, TMPDIR
- **Prefix:** MAENIFOLD_*, LC_*

All other environment variables (including AZURE_OPENAI_API_KEY, GITHUB_TOKEN, AWS_*, SSH_AUTH_SOCK, etc.) are stripped before the agent runs. This prevents accidental secret leakage to scheduled prompt logs.

The `Schedule action=create` and `Schedule action=update` responses include an `envScrub` field disclosing the active allowlist. To pass a specific environment variable to a scheduled run, prefix it with `MAENIFOLD_USER_ENV_` in the launching environment (planned follow-up; not yet implemented).

The launchd plist's `EnvironmentVariables` key remains intentionally sparse (`PATH` only). The actual scrub happens at fire time inside `maenifold schedule run` to give a single source of truth across launchd / systemd.

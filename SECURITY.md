# Security Policy - maenifold MCP

This document outlines security practices for the maenifold MCP Server project.

## Reporting Security Issues

For security vulnerabilities, please open a private issue or contact maintainers directly. Do not post security issues publicly.

## Secret Scanning

This repository uses **gitleaks** for automated secret detection to prevent accidental commits of sensitive data.

### Pre-commit Hook Setup

The repository includes a pre-commit hook that automatically scans for secrets before each commit:

```bash
# Enable the pre-commit hook (already configured)
git config core.hooksPath .githooks

# Verify hook is executable
chmod +x .githooks/pre-commit
```

### Manual Scanning

Run a manual scan anytime:

```bash
# Install gitleaks if not already installed
brew install gitleaks

# Run scan
gitleaks detect --source . --report-path gitleaks-report.json --redact

# Check git history for secrets
gitleaks detect --source . --log-opts="--all" --redact
```

## Configuration Security

### Environment Variables

All sensitive configuration is managed through environment variables:

- **MAENIFOLD_ROOT**: Base directory for workspace isolation
- **MAENIFOLD_DEBUG**: Enable debug logging (disable in production)
- **AGENT_ID**: Optional agent identifier (non-sensitive)

Never commit `.env` files or configuration with actual values.

### Database Security

- SQLite database uses parameterized queries throughout
- Foreign key constraints enforced
- WAL mode enabled for better concurrency
- No raw SQL concatenation

### Path Security

- Path traversal protection in `MarkdownIO.UriToPath()`
- All paths validated to stay within configured base directory
- Memory URIs sanitized before filesystem operations

## Dependencies

Current dependencies are from trusted sources:
- Microsoft.Extensions.Hosting (9.0.9)
- ModelContextProtocol (0.3.0-preview.4)
- Microsoft.Data.Sqlite (9.0.9)
- YamlDotNet (16.3.0)
- Markdig (0.42.0)
- Microsoft.ML.OnnxRuntime (1.23.0)

Keep dependencies updated and monitor for security advisories.

## MCP Server Security

### Input Validation
- All tool inputs validated through JSON deserialization
- Typed parameters with schema validation
- Enum parsing with proper error handling

### Authentication
- MCP server runs locally via stdio transport
- No network exposure by default
- Authentication handled by MCP client

## Build & Development

### Required Security Practices

1. **Never disable TreatWarningsAsErrors** - All warnings must be fixed
2. **Run tests before commits** - `dotnet test`
3. **Use CLI for testing** - Test tools via CLI before MCP integration
4. **Review logs** - Check console output doesn't leak sensitive data

### CI/CD Recommendations

For production deployments:

```yaml
# Example GitHub Actions secret scanning
name: Security Scan
on: [push, pull_request]
jobs:
  gitleaks:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run gitleaks
        uses: zricethezav/gitleaks-action@v2
        with:
          args: '--redact'
```

## Data Privacy

### Logging
- Console output limited to errors and essential information
- No PII logged by default
- Debug logging requires explicit environment variable

### Memory Files
- Stored locally in user's home directory
- No automatic cloud sync or external transmission
- User controls all memory content

## Security Checklist

Before each commit:
- [ ] Pre-commit hook passes (gitleaks scan)
- [ ] No hardcoded secrets or credentials
- [ ] No sensitive file paths or user data
- [ ] Dependencies up to date
- [ ] Tests passing

Before release:
- [ ] Security scan on full git history
- [ ] Dependency vulnerability check
- [ ] Documentation review for accidental secrets
- [ ] Production config validated

## False Positives

If gitleaks reports a false positive, you can:

1. Review the finding in `gitleaks-report.json`
2. If confirmed false positive, create `.gitleaks.toml`:

```toml
[allowlist]
description = "Allowlist for maenifold MCP"
paths = [
  # Add specific file paths to exclude
]
regexes = [
  # Add patterns for known false positives
]
```

## Contact

For security issues or questions about this project:

- **Repository**: <https://github.com/msbrettorg/maenifold>
- **Issues**: <https://github.com/msbrettorg/maenifold/issues> (for non-sensitive issues)

**Please do not report security vulnerabilities through public GitHub issues.** 
For sensitive security matters, please contact the maintainers directly through private channels or create a private security advisory on GitHub.

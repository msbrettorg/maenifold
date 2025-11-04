# Contributing to maenifold

Thank you for your interest in contributing! **maenifold** follows a unique design philosophy called the **Ma (間) Protocol**, which shapes how we approach code, security, testing, and architecture.

**Please read the [Design Philosophy](#design-philosophy) section below** to understand why certain patterns you might consider "bugs" or "missing features" are actually **intentional design decisions**.

---

## Branch Structure

maenifold uses a two-tier branch protection model:

```
feature-branch → dev → main
```

- **`main`**: Production-ready code, always stable
- **`dev`**: Integration branch for all features and fixes
- **`feature-branch`**: Your work branch

Both `main` and `dev` are protected branches that require pull requests.

## Workflow

### 1. Create a Feature Branch

Branch from `dev`:

```bash
git checkout dev
git pull origin dev
git checkout -b feature/your-feature-name
```

Branch naming conventions:
- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation updates
- `refactor/` - Code refactoring

### 2. Make Your Changes

```bash
# Make your changes
git add .
git commit -m "feat: Add your feature description"

# Push to your feature branch
git push origin feature/your-feature-name
```

### 3. Create Pull Request to `dev`

```bash
gh pr create --base dev --title "feat: Your feature description" --body "Description of changes"
```

Or use the GitHub web interface.

### 4. After Merge: `dev` to `main`

Once features are tested in `dev`, create a PR to `main`:

```bash
gh pr create --base main --head dev --title "Release: Version X.Y.Z" --body "Summary of changes"
```

## Commit Message Format

Follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation changes
- `refactor:` - Code refactoring
- `test:` - Test updates
- `chore:` - Build/tooling changes

Examples:
```
feat: Add CLI interface documentation to README
fix: Resolve SQLite permission errors in CI tests
docs: Update tool name references to PascalCase
```

## Merge Conflict Resolution

If your PR has conflicts:

1. Update your branch with the target branch:
   ```bash
   git fetch origin
   git merge origin/dev  # or origin/main
   ```

2. Resolve conflicts in your editor

3. Commit and push:
   ```bash
   git add .
   git commit -m "Merge conflicts resolved"
   git push origin your-branch-name
   ```

## Code Standards

- Follow [.NET runtime coding guidelines](https://github.com/dotnet/runtime/blob/main/docs/coding-guidelines/coding-style.md)
- Allman style braces
- Four spaces for indentation
- PascalCase for tool names
- All configuration in config files, not hardcoded

## Testing

Run tests before submitting PR:

```bash
dotnet test
```

For release builds:

```bash
dotnet build -c Release
```

---

## Design Philosophy

### The Ma (間) Protocol

Maenifold is built on the **Ma (間) Protocol** - a philosophy that creates space for intelligence to emerge by **resisting unnecessary features and abstractions**. "Ma" (間) is a Japanese concept meaning "negative space" - the idea that **absence is a feature**.

#### Core Principles

**1. NO FAKE AI** - Every decision we make removes a decision the LLM could make
- ❌ Don't add retry logic or fallback strategies
- ❌ Don't implement "smart" error recovery
- ✅ **Let errors propagate to the LLM with complete information**

**2. NO UNNECESSARY ABSTRACTIONS** - Abstractions create cognitive load
- ❌ No interfaces for single implementations
- ❌ No factory patterns or dependency injection frameworks
- ✅ **Direct calls over indirection**

**3. NO FAKE TESTS** - Mocks create false confidence
- ❌ No mocks, no stubs, no test doubles
- ✅ **Use real SQLite databases and real file systems**

**4. NO FAKE SECURITY** - This is a local personal knowledge system
- ❌ No path validation or artificial resource limits
- ✅ **Use prepared statements for SQL (the only security measure needed)**
- ✅ **Trust the user to manage their own system**

### Common "Bugs" That Are Actually Features

When reviewing code, you might notice:

| What You See | Why It's Intentional |
|--------------|---------------------|
| No path validation with `Path.GetFullPath()` | **NO FAKE SECURITY** - OS provides real boundaries |
| No artificial limits on parameters | **NO FAKE SECURITY** - trust the user |
| No retry logic for failed operations | **NO FAKE AI** - errors must propagate |
| Debounce timers "coalescing" rapid events | **INTENTIONAL** - batching is the feature |
| No dependency injection | **NO UNNECESSARY ABSTRACTIONS** |
| Tests use real SQLite | **NO FAKE TESTS** - real systems find real bugs |

### What We Want Fixed

✅ **Do contribute:**
- Bugs that violate our own principles (e.g., swallowed exceptions)
- Performance improvements with measurements
- Features that amplify LLM intelligence
- Documentation and clarity improvements

❌ **Don't contribute:**
- Validation/sanitization "for security"
- Retry logic or error recovery
- Interfaces for single implementations
- Mocked tests replacing real tests
- Artificial resource limits

### Further Reading

- **`CLAUDE.md`** - Full project overview and Ma Protocol details
- **`/docs/MA_MANIFESTO.md`** - Deep dive into philosophy
- **`/docs/WHAT_WE_DONT_DO.md`** - Rejected features
- **`/docs/TESTING_PHILOSOPHY.md`** - Why we don't mock
- **`/docs/SECURITY_PHILOSOPHY.md`** - Security for personal systems

---

## Questions?

If you're unsure whether your contribution aligns with Ma Protocol:
1. Read the philosophy docs above
2. Open an issue to discuss the approach
3. Reference specific Ma Protocol principles in your PR

Open an issue or reach out to the maintainers.

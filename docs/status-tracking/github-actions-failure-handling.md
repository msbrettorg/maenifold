# GitHub Actions Build Failure Handling and Error Recovery Patterns

**Research Date:** 2025-10-28
**Source:** Official GitHub Documentation (docs.github.com only)
**Ma Protocol Compliance:** Only documented official behavior, no assumptions

---

## Table of Contents

1. [Workflow-Level Failure Handling](#workflow-level-failure-handling)
2. [Job-Level Failure Handling](#job-level-failure-handling)
3. [Step-Level Failure Handling](#step-level-failure-handling)
4. [Detecting and Responding to Failures](#detecting-and-responding-to-failures)
5. [Notification Options](#notification-options)
6. [Retry and Re-run Patterns](#retry-and-re-run-patterns)
7. [Debugging Failed Runs](#debugging-failed-runs)
8. [Triggering Workflows on Failures](#triggering-workflows-on-failures)
9. [Status Badges](#status-badges)
10. [Complete Examples](#complete-examples)

---

## Workflow-Level Failure Handling

### Status Check Functions

GitHub Actions provides four status check functions for conditional expressions in workflows:

#### success()

> "Returns `true` when all previous steps have succeeded."

**Source:** https://docs.github.com/en/actions/learn-github-actions/expressions

**Example:**
```yaml
steps:
  - name: The job has succeeded
    if: ${{ success() }}
```

#### failure()

> "Returns `true` when any previous step of a job fails."

**Important:** "If you have a chain of dependent jobs, `failure()` returns true if any ancestor job fails."

**Source:** https://docs.github.com/en/actions/learn-github-actions/expressions

**Example:**
```yaml
steps:
  - name: Failing step
    id: demo
    run: exit 1
  - name: The demo step has failed
    if: ${{ failure() && steps.demo.conclusion == 'failure' }}
```

#### always()

> "Causes the step to always execute, and returns `true`, even when canceled."

**Caveat:** "Avoid using `always` for any task that could suffer from a critical failure."

**Source:** https://docs.github.com/en/actions/learn-github-actions/expressions

**Example:**
```yaml
steps:
  - name: Always runs
    if: ${{ always() }}
```

#### cancelled()

> "Returns `true` if the workflow was canceled."

**Source:** https://docs.github.com/en/actions/learn-github-actions/expressions

**Example:**
```yaml
if: ${{ cancelled() }}
```

---

## Job-Level Failure Handling

### continue-on-error (Job Level)

**Syntax:** `jobs.<job_id>.continue-on-error`

> "Prevents a job from failing when a step fails. Set to true to allow a job to pass when this step fails."

**Source:** https://docs.github.com/en/actions/using-workflows/workflow-syntax-for-github-actions

**Example:**
```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    continue-on-error: true
    steps:
      - run: npm test
```

### fail-fast Strategy

**Syntax:** `jobs.<job_id>.strategy.fail-fast`

> "When set to true, GitHub cancels all in-progress jobs if any matrix job fails. Default: true"

**Source:** https://docs.github.com/en/actions/using-workflows/workflow-syntax-for-github-actions

**Example:**
```yaml
jobs:
  test:
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, windows-latest]
        node: [14, 16, 18]
```

### timeout-minutes (Job Level)

**Syntax:** `jobs.<job_id>.timeout-minutes`

> "The maximum number of minutes to allow a job to run before GitHub automatically cancels it."

**Source:** https://docs.github.com/en/actions/using-workflows/workflow-syntax-for-github-actions

**Example:**
```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    timeout-minutes: 30
```

### Job Dependencies and Failure Propagation

**Syntax:** `jobs.<job_id>.needs`

> "Use `jobs.<job_id>.needs` to identify any jobs that must complete successfully before this job will run."

**Failure Behavior:** "If a job fails or is skipped, all jobs that need it are skipped unless the jobs use a conditional expression that causes the job to continue."

**Chain Failures:** "If a run contains a series of jobs that need each other, a failure or skip applies to all jobs in the dependency chain from the point of failure or skip onwards."

**Source:** https://docs.github.com/en/actions/using-jobs/using-jobs-in-a-workflow

**Example - Sequential Jobs:**
```yaml
jobs:
  job1:
    runs-on: ubuntu-latest
    steps:
      - run: exit 0

  job2:
    needs: job1
    runs-on: ubuntu-latest
    steps:
      - run: exit 0

  job3:
    needs: [job1, job2]
    runs-on: ubuntu-latest
    steps:
      - run: exit 0
```

**Example - Running After Failures:**
```yaml
jobs:
  job1:
    runs-on: ubuntu-latest
    steps:
      - run: exit 1

  job2:
    needs: job1
    runs-on: ubuntu-latest
    steps:
      - run: exit 0

  job3:
    if: ${{ always() }}
    needs: [job1, job2]
    runs-on: ubuntu-latest
    steps:
      - run: echo "This runs regardless of job1 and job2 status"
```

> "If you would like a job to run even if a job it is dependent on did not succeed, use the `always()` conditional expression in `jobs.<job_id>.if`."

**Source:** https://docs.github.com/en/actions/using-jobs/using-jobs-in-a-workflow

---

## Step-Level Failure Handling

### continue-on-error (Step Level)

**Syntax:** `jobs.<job_id>.steps[*].continue-on-error`

> "Prevents a job from failing when a step fails. Set to true to allow a job to pass when this step fails."

**Source:** https://docs.github.com/en/actions/using-workflows/workflow-syntax-for-github-actions

**Example:**
```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - name: Test
        run: npm test
        continue-on-error: true

      - name: Deploy
        run: npm run deploy
```

**Dynamic continue-on-error:**
```yaml
steps:
  - name: Test
    run: npm test
    continue-on-error: ${{ fromJSON(env.CONTINUE_ON_ERROR) }}
```

### timeout-minutes (Step Level)

**Syntax:** `jobs.<job_id>.steps[*].timeout-minutes`

> "The maximum number of minutes to allow a step to run before GitHub automatically cancels it."

**Source:** https://docs.github.com/en/actions/using-workflows/workflow-syntax-for-github-actions

**Example:**
```yaml
steps:
  - name: Long running test
    run: npm test
    timeout-minutes: 10
```

### Conditional Step Execution

**Syntax:** `jobs.<job_id>.steps[*].if`

**Example:**
```yaml
steps:
  - name: Build
    id: build
    run: npm run build

  - name: Upload artifacts on failure
    if: ${{ failure() }}
    uses: actions/upload-artifact@v4
    with:
      name: build-logs
      path: logs/
```

---

## Detecting and Responding to Failures

### Steps Context

The `steps` context provides two properties to determine step outcomes:

#### steps.<step_id>.outcome

> "The result of a completed step before `continue-on-error` is applied. Possible values are `success`, `failure`, `cancelled`, or `skipped`."

**Source:** https://docs.github.com/en/actions/reference/workflows-and-actions/contexts

#### steps.<step_id>.conclusion

> "The result of a completed step after `continue-on-error` is applied."

**Note:** When a step with `continue-on-error` fails, the outcome is `failure`, but the conclusion becomes `success`.

**Source:** https://docs.github.com/en/actions/reference/workflows-and-actions/contexts

**Example:**
```yaml
steps:
  - name: Test
    id: test_step
    run: npm test
    continue-on-error: true

  - name: Check test result
    if: ${{ steps.test_step.outcome == 'failure' }}
    run: echo "Tests failed but continuing..."
```

### Needs Context

**Syntax:** `needs.<job_id>.result`

> "The result of a job that the current job depends on. Possible values are `success`, `failure`, `cancelled`, or `skipped`."

**Source:** https://docs.github.com/en/actions/reference/workflows-and-actions/contexts

**Example:**
```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: exit 1

  deploy:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - run: echo "Deploying..."

  debug:
    needs: [build, deploy]
    if: ${{ failure() }}
    runs-on: ubuntu-latest
    steps:
      - run: echo "Build result: ${{ needs.build.result }}"
      - run: echo "Deploy result: ${{ needs.deploy.result }}"
```

### Job Context

**Syntax:** `job.status`

> "The current status of the job. Possible values are `success`, `failure`, or `cancelled`."

**Source:** https://docs.github.com/en/actions/reference/workflows-and-actions/contexts

---

## Notification Options

### Email Notifications

> "If you enable email or web notifications for GitHub Actions, you'll receive a notification when any workflow runs that you've triggered have completed."

**Source:** https://docs.github.com/en/actions/monitoring-and-troubleshooting-workflows/notifications-for-workflow-runs

#### Failure-Only Notifications

> "You can also choose to receive a notification only when a workflow run has failed."

**Configuration Steps:**
1. Navigate to notification settings
2. Under System → Actions, select "Email" from the dropdown
3. Optionally, select "Only notify for failed workflows" from the dropdown
4. Click Save

**Source:** https://docs.github.com/en/actions/monitoring-and-troubleshooting-workflows/notifications-for-workflow-runs

### Scheduled Workflow Notifications

**Rules for scheduled workflows:**

- Initial creator receives notifications by default
- When a user modifies the cron syntax, subsequent notifications route to that user instead
- Re-enabling a disabled scheduled workflow sends notifications to whoever performed the re-enablement

**Source:** https://docs.github.com/en/actions/monitoring-and-troubleshooting-workflows/notifications-for-workflow-runs

### GitHub UI Monitoring

> "You can also see the status of workflow runs on a repository's Actions tab"

**Source:** https://docs.github.com/en/actions/monitoring-and-troubleshooting-workflows/notifications-for-workflow-runs

### Third-Party Integrations

#### Slack Integration

> "The Slack + GitHub app allows you to open and close issues, comment on issues and pull requests, approve deployments, and receive personal pings on Slack when mentioned as part of any GitHub notifications."

**Source:** https://docs.github.com/en/get-started/exploring-integrations/featured-github-integrations

**Note:** More information is available in the integrations/slack repository README.

#### Webhook Integration

> "You can parse webhook payloads and integrate them into tools your team uses like Slack, Microsoft Teams, Splunk, or email."

**Source:** https://docs.github.com/en/get-started/exploring-integrations/featured-github-integrations

---

## Retry and Re-run Patterns

### Official Re-run Capability

> "You can re-run a workflow run, all failed jobs in a workflow run, or specific jobs in a workflow run up to 30 days after its initial run."

**Source:** https://docs.github.com/en/actions/managing-workflow-runs/re-running-workflows-and-jobs

### Re-run Methods

#### Via GitHub Web Interface

**Steps:**
1. Navigate to the repository, click Actions
2. Select the workflow
3. Click the workflow run name
4. In the upper-right corner, select the "Re-run jobs" dropdown menu
5. Click "Re-run failed jobs" (or "Re-run all jobs")

**Source:** https://docs.github.com/en/actions/managing-workflow-runs/re-running-workflows-and-jobs

#### Via GitHub CLI

**Re-run failed jobs:**
```bash
gh run rerun RUN_ID --failed
```

**Re-run specific job:**
```bash
gh run rerun --job JOB_ID
```

**Enable debug logging:**
```bash
gh run rerun RUN_ID --failed --debug
```

> "Add the `--debug` flag to enable runner diagnostic logging and step debug logging for the re-run."

**Source:** https://docs.github.com/en/actions/managing-workflow-runs/re-running-workflows-and-jobs

### Re-run Behavior and Limitations

> "Re-run workflows use the privileges of the actor who initially triggered the workflow, not the privileges of the actor who initiated the re-run."

> "The workflow will also use the same GITHUB_SHA (commit SHA) and GITHUB_REF (git ref) of the original event that triggered the workflow run."

> "People with write permissions to a repository can re-run workflows in the repository."

**Source:** https://docs.github.com/en/actions/managing-workflow-runs/re-running-workflows-and-jobs

### Viewing Previous Attempts

> "Select the 'Latest' dropdown menu next to the run name to access and review earlier run attempts within the 30-day window."

**Source:** https://docs.github.com/en/actions/managing-workflow-runs/re-running-workflows-and-jobs

### Note on Automatic Retries

**IMPORTANT:** GitHub Actions does not provide built-in automatic retry functionality for failed jobs or steps. All retries must be initiated manually via the UI, CLI, or API, or implemented within workflow steps using custom scripting.

---

## Debugging Failed Runs

### Viewing Failure Logs

> "If your workflow run fails, you can see which step caused the failure and review the failed step's build logs to troubleshoot."

**Automatic Expansion:** "Any failed steps are automatically expanded to display the results."

**Source:** https://docs.github.com/en/actions/monitoring-and-troubleshooting-workflows/using-workflow-run-logs

### Official Debugging Pattern

**Steps to diagnose failures:**

1. Navigate to the Actions tab in your repository
2. Select the specific workflow from the left sidebar
3. Click the failed workflow run name
4. Click the job under "Jobs" or in the visualization graph
5. Review the automatically expanded failed steps

**Source:** https://docs.github.com/en/actions/monitoring-and-troubleshooting-workflows/using-workflow-run-logs

### Additional Debugging Features

**Execution Timing:** You can see how long each step took to run

**Permalinks:** "You can copy a permalink to a specific line in the log file to share with your team"

**Log Search:** "You can also search expanded logs using the 'Search logs' box in the upper-right corner of the log output."

**Source:** https://docs.github.com/en/actions/monitoring-and-troubleshooting-workflows/using-workflow-run-logs

### Debug Logging on Re-runs

> "Both UI and CLI support optional debug logging enablement during re-runs to capture additional diagnostic information."

**Source:** https://docs.github.com/en/actions/managing-workflow-runs/re-running-workflows-and-jobs

---

## Triggering Workflows on Failures

### workflow_run Event

> "The `workflow_run` event allows you to execute a workflow based on execution or completion of another workflow."

**Activity Types:** `completed`, `requested`, `in_progress`

> "This event occurs when a workflow run is requested or completed"

**Source:** https://docs.github.com/en/actions/using-workflows/events-that-trigger-workflows

**Example:**
```yaml
on:
  workflow_run:
    workflows: [Run Tests]
    types:
      - completed
```

#### Multiple Workflow Triggers

> "If you specify multiple `workflows` for the `workflow_run` event, only one of the workflows needs to run."

**Source:** https://docs.github.com/en/actions/using-workflows/events-that-trigger-workflows

#### Conditional Execution Based on Conclusion

> "Use `github.event.workflow_run.conclusion` to run jobs based on the triggering workflow's result"

**Source:** https://docs.github.com/en/actions/using-workflows/events-that-trigger-workflows

**Example - Run on Success:**
```yaml
on:
  workflow_run:
    workflows: [Build]
    types: [completed]

jobs:
  on-success:
    if: github.event.workflow_run.conclusion == 'success'
    runs-on: ubuntu-latest
    steps:
      - run: echo "Build succeeded!"
```

**Example - Run on Failure:**
```yaml
on:
  workflow_run:
    workflows: [Build]
    types: [completed]

jobs:
  on-failure:
    if: github.event.workflow_run.conclusion == 'failure'
    runs-on: ubuntu-latest
    steps:
      - run: echo "Build failed!"
      - name: Send notification
        run: |
          curl -X POST -H 'Content-type: application/json' \
            --data '{"text":"Build failed!"}' \
            ${{ secrets.SLACK_WEBHOOK }}
```

#### Security Context

> "The workflow started by the `workflow_run` event is able to access secrets and write tokens, even if the previous workflow was not"

**Source:** https://docs.github.com/en/actions/using-workflows/events-that-trigger-workflows

---

## Status Badges

### Creating Status Badges

> "A status badge shows whether a workflow is currently failing or passing."

**Badge URL Format:**
```
https://github.com/OWNER/REPOSITORY/actions/workflows/WORKFLOW-FILE/badge.svg
```

**Source:** https://docs.github.com/en/actions/monitoring-and-troubleshooting-workflows/adding-a-workflow-status-badge

### Badge Behavior

**Default Branch:** "By default, badges display the status of your default branch. If there are no workflow runs on your default branch, it will display the status of the most recent run across all branches."

**Source:** https://docs.github.com/en/actions/monitoring-and-troubleshooting-workflows/adding-a-workflow-status-badge

### Creating via UI

**Steps:**
1. Under your repository name, click Actions
2. In the left sidebar, click the workflow you want to see
3. On the right side of the page, next to the "Filter workflow runs" field, click to display a dropdown menu
4. Click "Create status badge"

**Source:** https://docs.github.com/en/actions/monitoring-and-troubleshooting-workflows/adding-a-workflow-status-badge

### Badge Filtering

**By Branch:**
```markdown
![example workflow](https://github.com/OWNER/REPO/actions/workflows/WORKFLOW-FILE/badge.svg?branch=BRANCH-NAME)
```

**By Event:**
```markdown
![example workflow](https://github.com/OWNER/REPO/actions/workflows/WORKFLOW-FILE/badge.svg?event=push)
```

**Source:** https://docs.github.com/en/actions/monitoring-and-troubleshooting-workflows/adding-a-workflow-status-badge

### Private Repository Note

> "Workflow badges in a private repository are not accessible externally, so you won't be able to embed them or link to them from an external site."

**Source:** https://docs.github.com/en/actions/monitoring-and-troubleshooting-workflows/adding-a-workflow-status-badge

---

## Complete Examples

### Example 1: Comprehensive Failure Handling

```yaml
name: Build and Deploy
on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    timeout-minutes: 30
    strategy:
      fail-fast: false
      matrix:
        node: [14, 16, 18]
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: ${{ matrix.node }}

      - name: Install dependencies
        run: npm ci

      - name: Run tests
        id: tests
        run: npm test
        timeout-minutes: 10
        continue-on-error: true

      - name: Upload test results on failure
        if: ${{ steps.tests.outcome == 'failure' }}
        uses: actions/upload-artifact@v4
        with:
          name: test-results-node-${{ matrix.node }}
          path: test-results/

      - name: Fail if tests failed
        if: ${{ steps.tests.outcome == 'failure' }}
        run: exit 1

  deploy:
    needs: build
    runs-on: ubuntu-latest
    if: ${{ success() && github.ref == 'refs/heads/main' }}
    steps:
      - name: Deploy
        run: echo "Deploying application..."

  notify:
    needs: [build, deploy]
    runs-on: ubuntu-latest
    if: ${{ always() }}
    steps:
      - name: Check build status
        run: |
          echo "Build status: ${{ needs.build.result }}"
          echo "Deploy status: ${{ needs.deploy.result }}"

      - name: Notify on failure
        if: ${{ needs.build.result == 'failure' || needs.deploy.result == 'failure' }}
        run: |
          echo "Workflow failed - send notification here"
```

### Example 2: Recovery Workflow Using workflow_run

**Main workflow (build.yml):**
```yaml
name: Build
on: [push]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: npm ci
      - run: npm run build
      - run: npm test
```

**Recovery workflow (on-build-failure.yml):**
```yaml
name: On Build Failure
on:
  workflow_run:
    workflows: [Build]
    types: [completed]

jobs:
  notify-failure:
    if: ${{ github.event.workflow_run.conclusion == 'failure' }}
    runs-on: ubuntu-latest
    steps:
      - name: Send Slack notification
        run: |
          curl -X POST -H 'Content-type: application/json' \
            --data '{
              "text": "Build failed for ${{ github.event.workflow_run.head_branch }}",
              "blocks": [
                {
                  "type": "section",
                  "text": {
                    "type": "mrkdwn",
                    "text": "Build failed!\n*Branch:* ${{ github.event.workflow_run.head_branch }}\n*Commit:* ${{ github.event.workflow_run.head_sha }}\n*Run:* ${{ github.event.workflow_run.html_url }}"
                  }
                }
              ]
            }' \
            ${{ secrets.SLACK_WEBHOOK }}

      - name: Create issue on repeated failures
        uses: actions/github-script@v7
        with:
          script: |
            // Custom logic to create issue if build fails multiple times
            console.log('Creating issue for repeated failure...')
```

### Example 3: Programmatic Error Messages

```yaml
name: Error Handling
on: [push]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Run validation
        run: |
          if [ ! -f "config.json" ]; then
            echo "::error file=config.json,line=1,title=Missing Config::Configuration file not found"
            exit 1
          fi

      - name: Run tests with annotations
        run: |
          npm test || {
            echo "::error title=Test Failure::Tests failed with exit code $?"
            exit 1
          }
```

### Example 4: Cleanup Job That Always Runs

```yaml
name: Build with Cleanup
on: [push]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build
        run: npm run build
      - name: Test
        run: npm test

  cleanup:
    needs: build
    runs-on: ubuntu-latest
    if: ${{ always() }}
    steps:
      - name: Cleanup resources
        run: echo "Cleaning up resources..."

      - name: Report failure
        if: ${{ needs.build.result == 'failure' }}
        run: |
          echo "Build failed - performing failure cleanup"
          # Additional cleanup for failures
```

---

## Workflow Command Reference

### Error Command

**Syntax:**
```bash
echo "::error file={name},line={line},endLine={endLine},title={title}::{message}"
```

**Example (Bash):**
```bash
echo "::error file=app.js,line=1,col=5,endColumn=7,title=YOUR-TITLE::Missing semicolon"
```

**Example (PowerShell):**
```powershell
Write-Output "::error file=app.js,line=1,col=5,endColumn=7,title=YOUR-TITLE::Missing semicolon"
```

**Parameters:**

| Parameter | Required | Default |
|-----------|----------|---------|
| `title` | No | None |
| `file` | No | `.github` |
| `col` | No | None |
| `line` | No | `1` |
| `endLine` | No | `1` |

**Source:** https://docs.github.com/en/actions/using-workflows/workflow-commands-for-github-actions

### Programmatic Failure

> "The toolkit function `core.setFailed` is a shortcut for `::error` and `exit 1`"

**Source:** https://docs.github.com/en/actions/using-workflows/workflow-commands-for-github-actions

---

## Key Takeaways

### Failure Handling Strategies

1. **Step-level:** Use `continue-on-error` to allow individual steps to fail without failing the job
2. **Job-level:** Use `needs` with conditional expressions (`if: always()`, `if: failure()`) to control job execution
3. **Matrix-level:** Use `fail-fast: false` to prevent one matrix job from canceling others
4. **Workflow-level:** Use `workflow_run` event to trigger recovery workflows

### Notification Methods

1. **Built-in:** Email notifications via GitHub (with failure-only filtering)
2. **UI:** GitHub Actions tab shows status of all runs
3. **Badges:** Visual status indicators in README files
4. **Integrations:** Slack, Microsoft Teams via webhooks or apps
5. **Custom:** webhook-based integrations for any service

### Retry Capabilities

1. **Manual Re-runs:** Via UI or CLI, up to 30 days after initial run
2. **Selective Re-runs:** Can re-run only failed jobs
3. **Debug Mode:** Optional enhanced logging for re-runs
4. **No Automatic Retries:** Must implement custom retry logic in steps if needed

### Debugging Approach

1. Failed steps automatically expand in UI
2. Logs show execution time for each step
3. Permalink sharing for specific log lines
4. Log search capability
5. Debug logging available on re-runs

---

## Summary of Official Capabilities

**GitHub Actions officially supports:**

✅ Continue-on-error (step and job level)
✅ Conditional execution based on failure status
✅ Manual re-running of failed jobs (UI, CLI, API)
✅ Email notifications with failure-only filtering
✅ Status badges showing workflow status
✅ Triggering workflows on other workflow failures
✅ Failure detection via contexts (steps, needs, job)
✅ Timeout controls at job and step levels
✅ Debug logging for troubleshooting
✅ Job dependency chains with failure propagation control

**GitHub Actions does NOT officially support:**

❌ Automatic retries (must be implemented manually)
❌ Built-in exponential backoff
❌ Automatic rollback on failure
❌ Built-in failure rate limiting

---

## Documentation Sources

All information in this document was extracted from official GitHub documentation:

1. **Workflow Syntax:** https://docs.github.com/en/actions/using-workflows/workflow-syntax-for-github-actions
2. **Re-running Workflows:** https://docs.github.com/en/actions/managing-workflow-runs/re-running-workflows-and-jobs
3. **Notifications:** https://docs.github.com/en/actions/monitoring-and-troubleshooting-workflows/notifications-for-workflow-runs
4. **Expressions:** https://docs.github.com/en/actions/learn-github-actions/expressions
5. **Workflow Commands:** https://docs.github.com/en/actions/using-workflows/workflow-commands-for-github-actions
6. **Workflow Logs:** https://docs.github.com/en/actions/monitoring-and-troubleshooting-workflows/using-workflow-run-logs
7. **Using Jobs:** https://docs.github.com/en/actions/using-jobs/using-jobs-in-a-workflow
8. **Contexts:** https://docs.github.com/en/actions/reference/workflows-and-actions/contexts
9. **Events:** https://docs.github.com/en/actions/using-workflows/events-that-trigger-workflows
10. **Status Badges:** https://docs.github.com/en/actions/monitoring-and-troubleshooting-workflows/adding-a-workflow-status-badge
11. **Job Conditions:** https://docs.github.com/en/actions/using-jobs/using-conditions-to-control-job-execution
12. **GitHub Integrations:** https://docs.github.com/en/get-started/exploring-integrations/featured-github-integrations

---

**Document Version:** 1.0
**Last Updated:** 2025-10-28
**Compliance:** Ma Protocol (No assumptions, only documented official behavior)

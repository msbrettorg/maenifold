# Maenifold Testing Retrospective

Run this at the end of every maenifold testing, UAT, or debugging cycle.

## Questions

1. What behavior was being proven?
2. What exact commands, app paths, or UI interactions were used?
3. What failed, if anything, and what was the exact error text?
4. Was the failure caused by the product, the test harness, environment/setup, stale binaries, or incorrect assumptions?
5. Which canonical source resolved the issue?
6. What should the next agent do differently?
7. Does `assets/integrations/skills/maenifold-testing/SKILL.md` or a reference file need an update?

## Update rule

Update the skill when the lesson changes future behavior. Good updates are concrete:

- a command to run before a known failure
- a script that is now canonical
- a source file that must be read before changing a surface
- an error signature with the documented resolution
- a cleanup rule tied to exact PIDs, ports, or artifacts

Do not add a work log. Do not paste large logs. Keep the skill focused on repeatable testing behavior.

## Suggested note format

```markdown
### <short lesson title>

- **Observed:** <exact symptom or failure>
- **Cause:** <root cause, with source>
- **Use next time:** <specific command, check, or workflow change>
- **Skill update:** <file/section changed, or "none: no reusable lesson">
```

# T-125.3 acceptance evidence

No dedicated desktop QA screenshot/evidence directory exists in `apps/desktop`, and the existing `assets/screenshot.png` is the product README image rather than a test artifact. Acceptance evidence for the red-team remediation is therefore stored next to the targeted regression suite.

## Covered blockers

1. **Search input spacebar text entry preserved**
   - Automated by `does not block spacebar text entry in the search input`
   - Confirms the search field no longer prevents default on `Space`, so multi-word queries remain typeable.

2. **Keyboard-only search/add flow still works**
   - Automated by `supports keyboard listbox navigation and submit without trapping focus`
   - Confirms keyboard navigation (`ArrowDown`) and keyboard activation (`Enter`, `Space`) still work from the listbox/options path.

3. **Persisted recents/pins survive initial empty/loading catalog**
   - Automated by `preserves persisted quick-add history until the catalog has really loaded`
   - Confirms persisted Quick Add history remains intact before catalog load, then reappears once real discovery data is present.

## Validation commands

- `cd apps/desktop && npx vitest run src/components/__tests__/ToolNodeFields.test.ts src/stores/__tests__/node-palette.test.ts`
- `cd apps/desktop && npx vue-tsc --noEmit`

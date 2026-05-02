# Frontend Build and Desktop Packaging

Use this reference for Vue, Tailwind, shadcn-vue/Reka, desktop build/package, design lint, component audit, and frontend verification failures.

## Build commands

From `apps/desktop`:

```bash
npm run build
npm run package
npx vue-tsc --noEmit
```

Merge-bound frontend verification uses `npm run build` because it runs the full chain: `vue-tsc`, design lint, component audit, and Vite build. Narrower changed-file scopes can hide regressions until package time.

After packaging, UI behavior still requires packaged Electron UAT through `scripts/uat-launch.sh`; typecheck and screenshots are not enough.

## node_modules/.bin failure

Known signature:

```text
sh: vue-tsc: command not found
```

Before dispatching blue-team or running `npm run build`, confirm:

```bash
test -x apps/desktop/node_modules/.bin/vue-tsc
```

If `.bin` is missing or empty:

```bash
cd apps/desktop && npm install
```

This restores package binary symlinks and runs Electron rebuild hooks.

## Design and component audit

- Use shadcn-vue/Reka wrappers and Tailwind token classes.
- No raw controls, style blocks, local CSS files, or `btn-*` classes.
- Keep Vue/TS files under the repo's LOC limit.
- Hex literals fail design lint. If an upstream literal is required, encode it so the audit regex does not treat it as a design token and document the upstream source.

## Electron renderer assumptions

Electron renderer/preload/CSP assumptions must be checked against actual sandboxed Electron code. `process.env` and Node APIs are not automatically available in the renderer.

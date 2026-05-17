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

## Vitest + jsdom test-setup gaps (iter-14 lessons)

Adding media or DOM-API-heavy dependencies to `apps/desktop/` requires updates to `apps/desktop/src/test/setup.ts`. The jsdom environment is intentionally minimal — anything beyond core DOM needs a polyfill.

**Plyr / video / audio components**: instantiating Plyr (or any player that touches `window.matchMedia` or `TextTrack` at construction) triggers two jsdom gaps:

- `window.matchMedia is not a function` at module-load time (Plyr reads `prefers-reduced-motion` early)
- `ReferenceError: TextTrack is not defined` at player-build time

Polyfill pattern for `src/test/setup.ts`:

```ts
if (typeof window !== 'undefined' && typeof window.matchMedia !== 'function') {
  Object.defineProperty(window, 'matchMedia', {
    configurable: true,
    writable: true,
    value: (query: string): MediaQueryList => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    }),
  });
}
```

**Constructable mocks**: classes invoked with `new` in production code (e.g., `new Plyr(el, opts)`) cannot be mocked with arrow functions — arrow functions are not constructable. Pattern:

```ts
const { Plyr, plyrDestroy } = vi.hoisted(() => {
  const destroy = vi.fn();
  const constructor = vi.fn(function (this: { destroy: typeof destroy }, _el: HTMLMediaElement) {
    this.destroy = destroy;
  });
  return { Plyr: constructor, plyrDestroy: destroy };
});
vi.mock('plyr', () => ({ default: Plyr }));
```

Arrow-function mocks fail at runtime with `TypeError: (_el) => ({ destroy }) is not a constructor`.

**Sibling-component contamination**: if a parent component (e.g., `ResourcePreview`) renders a media-using child (e.g., `AudioBlock`), the PARENT test file ALSO needs `vi.mock('plyr', ...)` because the import resolution triggers the player's module-load side effects.

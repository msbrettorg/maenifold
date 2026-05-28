import { createElement } from "react";
import { renderToString } from "react-dom/server";
import { act, renderHook } from "@testing-library/react";

import { useTheme } from "./useTheme";

type MatchMediaResult = {
  matches: boolean;
  media: string;
  onchange: null;
  addListener: ReturnType<typeof vi.fn>;
  removeListener: ReturnType<typeof vi.fn>;
  addEventListener: ReturnType<typeof vi.fn>;
  removeEventListener: ReturnType<typeof vi.fn>;
  dispatchEvent: ReturnType<typeof vi.fn>;
};

function installLocalStorage(initialEntries: Record<string, string> = {}) {
  const store = new Map(Object.entries(initialEntries));
  const localStorageMock = {
    getItem: vi.fn((key: string) => store.get(key) ?? null),
    setItem: vi.fn((key: string, value: string) => {
      store.set(key, value);
    }),
    removeItem: vi.fn((key: string) => {
      store.delete(key);
    }),
    clear: vi.fn(() => {
      store.clear();
    }),
  };

  Object.defineProperty(window, "localStorage", {
    configurable: true,
    value: localStorageMock,
  });

  return localStorageMock;
}

function installMatchMedia(matches: boolean) {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    writable: true,
    value: vi.fn((query: string): MatchMediaResult => ({
      matches,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
}

async function flushEffects() {
  await act(async () => {
    await Promise.resolve();
  });
}

describe("useTheme", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    installLocalStorage();
    if (
      typeof window !== "undefined" &&
      window.localStorage &&
      typeof window.localStorage.clear === "function"
    ) {
      window.localStorage.clear();
    }
    if (typeof document !== "undefined" && document.documentElement) {
      document.documentElement.className = "";
    }
    installMatchMedia(false);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("reads the initial theme from localStorage before consulting system preference", async () => {
    installLocalStorage({ theme: "dark" });
    installMatchMedia(false);

    const { result } = renderHook(() => useTheme());
    await flushEffects();

    expect(result.current.theme).toBe("dark");
    expect(result.current.isDark).toBe(true);
    expect(document.documentElement).toHaveClass("dark");
  });

  it("falls back to the system preference when localStorage is empty", async () => {
    installMatchMedia(true);

    const { result } = renderHook(() => useTheme());
    await flushEffects();

    expect(result.current.theme).toBe("dark");
    expect(result.current.isDark).toBe(true);
    expect(document.documentElement).toHaveClass("dark");
  });

  it("toggleTheme writes dark mode to localStorage and documentElement", async () => {
    const { result } = renderHook(() => useTheme());
    await flushEffects();

    act(() => {
      result.current.toggleTheme();
    });

    expect(result.current.theme).toBe("dark");
    expect(window.localStorage.getItem("theme")).toBe("dark");
    expect(document.documentElement).toHaveClass("dark");
  });

  it("toggleTheme writes light mode to localStorage and removes the dark class", async () => {
    installLocalStorage({ theme: "dark" });

    const { result } = renderHook(() => useTheme());
    await flushEffects();

    act(() => {
      result.current.toggleTheme();
    });

    expect(result.current.theme).toBe("light");
    expect(window.localStorage.getItem("theme")).toBe("light");
    expect(document.documentElement).not.toHaveClass("dark");
  });
});

describe("useTheme (SSR)", () => {
  it("is SSR-safe when window and document are unavailable", () => {
    const originalWindow = globalThis.window;
    const originalDocument = globalThis.document;
    // @ts-expect-error -- deliberate SSR simulation
    delete (globalThis as { window?: Window }).window;
    // @ts-expect-error -- deliberate SSR simulation
    delete (globalThis as { document?: Document }).document;

    function Probe() {
      useTheme();
      return null;
    }

    try {
      expect(() => renderToString(createElement(Probe))).not.toThrow();
    } finally {
      (globalThis as { window?: Window }).window = originalWindow;
      (globalThis as { document?: Document }).document = originalDocument;
    }
  });
});

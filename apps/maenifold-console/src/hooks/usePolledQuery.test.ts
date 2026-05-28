import { act, renderHook } from "@testing-library/react";

import { usePolledQuery } from "./usePolledQuery";

type Deferred<T> = {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (reason?: unknown) => void;
};

function createDeferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;

  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });

  return { promise, resolve, reject };
}

async function flushMicrotasks() {
  await act(async () => {
    await Promise.resolve();
  });
}

describe("usePolledQuery", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2024-01-01T00:00:00.000Z"));
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  it("fetches immediately on mount and surfaces loading then data state", async () => {
    const deferred = createDeferred<string>();
    const queryFn = vi.fn(() => deferred.promise);
    const { result } = renderHook(() =>
      usePolledQuery("pipeline", queryFn, { intervalMs: 2_000 }),
    );

    expect(queryFn).toHaveBeenCalledTimes(1);
    expect(result.current.isLoading).toBe(true);
    expect(result.current.data).toBeNull();
    expect(result.current.error).toBeNull();

    await act(async () => {
      deferred.resolve("initial");
      await deferred.promise;
    });

    expect(result.current.isLoading).toBe(false);
    expect(result.current.isRefreshing).toBe(false);
    expect(result.current.data).toBe("initial");
    expect(result.current.error).toBeNull();
    expect(result.current.lastUpdatedAt).toBe(Date.now());
  });

  it("polls at the configured interval and marks refreshes separately from initial load", async () => {
    const refresh = createDeferred<string>();
    const queryFn = vi
      .fn<() => Promise<string>>()
      .mockResolvedValueOnce("initial")
      .mockImplementationOnce(() => refresh.promise);
    const { result } = renderHook(() =>
      usePolledQuery("pipeline", queryFn, { intervalMs: 500 }),
    );

    await flushMicrotasks();

    expect(result.current.data).toBe("initial");
    expect(result.current.isLoading).toBe(false);
    expect(result.current.isRefreshing).toBe(false);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(499);
    });

    expect(queryFn).toHaveBeenCalledTimes(1);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });

    expect(queryFn).toHaveBeenCalledTimes(2);
    expect(result.current.isLoading).toBe(false);
    expect(result.current.isRefreshing).toBe(true);

    await act(async () => {
      refresh.resolve("next");
      await refresh.promise;
    });

    expect(result.current.data).toBe("next");
    expect(result.current.isRefreshing).toBe(false);
  });

  it("stops polling after unmount and clears the scheduled timer", async () => {
    const queryFn = vi.fn().mockResolvedValue("snapshot");
    const clearTimeoutSpy = vi.spyOn(window, "clearTimeout");
    const { unmount } = renderHook(() =>
      usePolledQuery("pipeline", queryFn, { intervalMs: 250 }),
    );

    await flushMicrotasks();
    expect(queryFn).toHaveBeenCalledTimes(1);

    unmount();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(2_000);
    });

    expect(queryFn).toHaveBeenCalledTimes(1);
    expect(clearTimeoutSpy).toHaveBeenCalled();
  });

  it("surfaces rejected requests as Error state", async () => {
    const failure = new Error("gateway exploded");
    const queryFn = vi.fn().mockRejectedValue(failure);
    const { result } = renderHook(() =>
      usePolledQuery("pipeline", queryFn, { intervalMs: 2_000 }),
    );

    await flushMicrotasks();

    expect(result.current.isLoading).toBe(false);
    expect(result.current.isRefreshing).toBe(false);
    expect(result.current.data).toBeNull();
    expect(result.current.error).toBe(failure);
    expect(result.current.lastUpdatedAt).toBeNull();
  });

  it("ignores an in-flight result after unmounting mid-request", async () => {
    const deferred = createDeferred<string>();
    const queryFn = vi.fn(() => deferred.promise);
    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});
    const { unmount } = renderHook(() =>
      usePolledQuery("pipeline", queryFn, { intervalMs: 2_000 }),
    );

    expect(queryFn).toHaveBeenCalledTimes(1);

    unmount();

    await act(async () => {
      deferred.resolve("late");
      await deferred.promise;
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(2_000);
    });

    expect(queryFn).toHaveBeenCalledTimes(1);
    expect(consoleErrorSpy).not.toHaveBeenCalled();
  });

  it("does not fetch or poll while disabled", async () => {
    const queryFn = vi.fn().mockResolvedValue("unused");
    const { result } = renderHook(() =>
      usePolledQuery("pipeline", queryFn, {
        enabled: false,
        intervalMs: 250,
        initialData: "seed",
      }),
    );

    expect(queryFn).not.toHaveBeenCalled();
    expect(result.current.data).toBe("seed");
    expect(result.current.isLoading).toBe(false);
    expect(result.current.isRefreshing).toBe(false);
    expect(result.current.error).toBeNull();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(2_000);
    });

    expect(queryFn).not.toHaveBeenCalled();
  });
});

describe("stability under re-render with unstable queryFn", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2024-01-01T00:00:00.000Z"));
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  it("does not restart initial loading or polling when only queryFn identity changes", async () => {
    const firstDeferred = createDeferred<string>();
    const secondDeferred = createDeferred<string>();
    let invocations = 0;

    const { result, rerender } = renderHook(
      ({ tick }: { tick: number }) => {
        const queryFn = () => {
          invocations += 1;

          if (invocations === 1) {
            return firstDeferred.promise;
          }

          if (invocations === 2) {
            return secondDeferred.promise;
          }

          return Promise.resolve(`extra-${tick}-${invocations}`);
        };

        return usePolledQuery("pipeline", queryFn, { intervalMs: 2_000 });
      },
      { initialProps: { tick: 0 } },
    );

    expect(invocations).toBe(1);
    expect(result.current.isLoading).toBe(true);

    act(() => {
      rerender({ tick: 1 });
      rerender({ tick: 2 });
    });

    expect(invocations).toBe(1);

    await act(async () => {
      firstDeferred.resolve("initial");
      await firstDeferred.promise;
    });

    expect(result.current.data).toBe("initial");
    expect(result.current.isLoading).toBe(false);

    act(() => {
      rerender({ tick: 3 });
    });

    expect(result.current.isLoading).toBe(false);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1_500);
    });

    expect(invocations).toBe(1);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(500);
    });

    expect(invocations).toBe(2);
    expect(result.current.isRefreshing).toBe(true);

    await act(async () => {
      secondDeferred.resolve("refreshed");
      await secondDeferred.promise;
    });

    expect(result.current.data).toBe("refreshed");
    expect(result.current.isRefreshing).toBe(false);
  });
});

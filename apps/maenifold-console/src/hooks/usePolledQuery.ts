import { useCallback, useEffect, useMemo, useRef, useState } from "react";

type UsePolledQueryOptions<TData> = {
  enabled?: boolean;
  intervalMs?: number;
  initialData?: TData;
};

type UsePolledQueryResult<TData> = {
  data: TData | null;
  error: Error | null;
  isLoading: boolean;
  isRefreshing: boolean;
  lastUpdatedAt: number | null;
  refetch: () => Promise<void>;
};

export function usePolledQuery<TData>(
  queryKey: string,
  queryFn: () => Promise<TData>,
  options: UsePolledQueryOptions<TData> = {},
): UsePolledQueryResult<TData> {
  const { enabled = true, intervalMs = 2_000, initialData } = options;
  const [data, setData] = useState<TData | null>(initialData ?? null);
  const [error, setError] = useState<Error | null>(null);
  const [isLoading, setIsLoading] = useState(Boolean(enabled && !initialData));
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [lastUpdatedAt, setLastUpdatedAt] = useState<number | null>(null);
  const requestIdRef = useRef(0);
  const timerRef = useRef<number | null>(null);
  const scheduleNextRef = useRef<(() => void) | null>(null);

  const clearTimer = useCallback(() => {
    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  }, []);

  const runQuery = useCallback(
    async (reason: "initial" | "poll" | "manual") => {
      if (!enabled) {
        return;
      }

      const requestId = requestIdRef.current + 1;
      requestIdRef.current = requestId;

      if (reason === "initial") {
        setIsLoading(true);
      } else {
        setIsRefreshing(true);
      }

      try {
        const nextData = await queryFn();

        if (requestId !== requestIdRef.current) {
          return;
        }

        setData(nextData);
        setError(null);
        setLastUpdatedAt(Date.now());
      } catch (caught) {
        if (requestId !== requestIdRef.current) {
          return;
        }

        setError(caught instanceof Error ? caught : new Error(String(caught)));
      } finally {
        if (requestId === requestIdRef.current) {
          setIsLoading(false);
          setIsRefreshing(false);
        }
      }
    },
    [enabled, queryFn],
  );

  useEffect(() => {
    clearTimer();

    if (!enabled) {
      setData(initialData ?? null);
      setError(null);
      setLastUpdatedAt(null);
      setIsLoading(false);
      setIsRefreshing(false);
      return undefined;
    }

    void runQuery("initial");

    const schedule = () => {
      clearTimer();
      timerRef.current = window.setTimeout(async () => {
        await runQuery("poll");
        schedule();
      }, intervalMs);
    };

    scheduleNextRef.current = schedule;
    schedule();

    return () => {
      requestIdRef.current += 1;
      clearTimer();
    };
  }, [clearTimer, enabled, initialData, intervalMs, queryKey, runQuery]);

  const refetch = useCallback(async () => {
    clearTimer();
    await runQuery("manual");
    if (enabled) {
      scheduleNextRef.current?.();
    }
  }, [clearTimer, enabled, runQuery]);

  return useMemo(
    () => ({
      data,
      error,
      isLoading,
      isRefreshing,
      lastUpdatedAt,
      refetch,
    }),
    [data, error, isLoading, isRefreshing, lastUpdatedAt, refetch],
  );
}

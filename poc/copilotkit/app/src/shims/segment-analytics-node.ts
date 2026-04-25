type TrackPayload = {
  event?: string;
  userId?: string;
  anonymousId?: string;
  properties?: Record<string, unknown>;
};

type AnalyticsOptions = {
  writeKey?: string;
};

export class Analytics {
  constructor(_options?: AnalyticsOptions) {}

  track(_payload: TrackPayload, callback?: (error?: Error) => void) {
    callback?.();
  }

  closeAndFlush(): Promise<void> {
    return Promise.resolve();
  }
}

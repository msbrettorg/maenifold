export type WrySubscriptionCallback<T = unknown> = (message: T) => void;

export interface WryShim {
  invoke(tool: string, args: unknown): Promise<unknown>;
  subscribe(channel: string, cb: WrySubscriptionCallback): () => void;
}

declare global {
  interface Window {
    maenifold?: WryShim;
  }
}

function getWryShim(): WryShim {
  if (typeof window === "undefined" || !window.maenifold) {
    throw new Error("window.maenifold is not available");
  }

  return window.maenifold;
}

export class WryBridge {
  private readonly shim: WryShim;

  constructor(shim: WryShim = getWryShim()) {
    this.shim = shim;
  }

  async invoke<T>(tool: string, args: unknown): Promise<T> {
    return (await this.shim.invoke(tool, args)) as T;
  }

  subscribe<T>(channel: string, cb: WrySubscriptionCallback<T>): () => void {
    return this.shim.subscribe(channel, (message) => cb(message as T));
  }
}

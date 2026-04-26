const AUTH_FAILED_MESSAGE =
  'Authorization failed. The daemon may have restarted — please reload the app.';

export function getHttpFetch(): typeof globalThis.fetch {
  return async (input: RequestInfo | URL, init: RequestInit = {}): Promise<Response> => {
    if (typeof window === 'undefined' || !window.electronAPI?.getAuthToken) {
      return globalThis.fetch(input, init);
    }

    const buildInit = async (): Promise<RequestInit> => {
      const token = await window.electronAPI?.getAuthToken?.();
      if (!token) throw new Error(AUTH_FAILED_MESSAGE);
      const headers = new Headers(init.headers);
      headers.set('Authorization', `Bearer ${token}`);
      return { ...init, headers };
    };

    let response = await globalThis.fetch(input, await buildInit());
    if (response.status !== 401) return response;

    response = await globalThis.fetch(input, await buildInit());
    if (response.status === 401) {
      throw new Error(`${AUTH_FAILED_MESSAGE} Retry the action after the daemon is ready.`);
    }
    return response;
  };
}

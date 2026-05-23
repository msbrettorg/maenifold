import { useMcpClient } from './useMcpClient';

const sharedMcpClients = new Map<string, ReturnType<typeof useMcpClient>>();

export function getSharedMcpClient(baseUrl?: string) {
  const key = baseUrl ?? '__default__';
  const existing = sharedMcpClients.get(key);
  if (existing) return existing;

  const client = useMcpClient(baseUrl);
  sharedMcpClients.set(key, client);
  return client;
}

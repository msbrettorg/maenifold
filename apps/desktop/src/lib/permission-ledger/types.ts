import type { RequestPermissionResponse } from '@agentclientprotocol/sdk';
import type { PermissionRequest } from '../types';

export type PermissionPromptPayload = {
  toolCall: PermissionRequest['toolCall'];
  options: PermissionRequest['options'];
  message?: string;
};

export type PermissionEvaluateRequestResult =
  | { autoDecided: true; response: RequestPermissionResponse }
  | { autoDecided: false; token: string; prompt: PermissionPromptPayload };

export type ChatNotificationSink = (message: {
  kind: 'plan-mode-write-blocked';
  path: string;
  reason: string;
}) => void;

let chatNotificationSink: ChatNotificationSink | null = null;

export function setChatNotificationSink(sink: ChatNotificationSink): void {
  chatNotificationSink = sink;
}

export function pushPlanModeWriteBlockedNotification(path: string): void {
  chatNotificationSink?.({
    kind: 'plan-mode-write-blocked',
    path,
    reason: 'Write blocked by plan mode',
  });
}

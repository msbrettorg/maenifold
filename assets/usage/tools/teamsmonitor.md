# TeamsMonitor

Controls the Teams monitor by enabling, disabling, reporting status, updating
configuration, detecting signed-in Teams identity, refreshing the signed-in token,
or deleting the agency-cowork-compatible Teams token cache entry.

## Parameters

- `action` (string, required): `enable`, `disable`, `status`, `configure`,
  `detect_identity`, `refresh`, or `sign_out`.
- `keyword` (string, optional): Trigger keyword for monitor matches.
- `listenScope` (string, optional): `wildcard`, `self`, or `custom`.
- `monitoredConversations` (string[], optional): Conversation IDs for custom scope.
- `replyBehavior` (string, optional): Outbound reply target behavior.
- `replyPrefix` (string, optional): Prefix applied to monitor replies.
- `dispatchTimeoutMinutes` (integer, optional): ACP dispatch timeout in minutes.
- `chatsvcRegion` (string, optional): Detected Teams chat service region.
- `dispatchAgent` (string, optional): ACP agent dispatched on match.

## Domain

MCP clients invoke this as `GatewayExecute` with domain `maenifold.system` and
command `TeamsMonitor`.

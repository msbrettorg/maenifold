<!-- Minimal local copy of the official ACP docs. Source: https://agentclientprotocol.com/protocol/extensibility -->

# Extensibility

> Adding custom data and capabilities

The Agent Client Protocol provides built-in extension mechanisms that allow implementations to add custom functionality while maintaining compatibility with the core protocol. These mechanisms ensure that Agents and Clients can innovate without breaking interoperability.

## The `_meta` Field

All types in the protocol include a `_meta` field with type `{ [key: string]: unknown }` that implementations can use to attach custom information.

Clients may propagate fields to the agent for correlation purposes, such as `requestId`.

The following root-level keys in `_meta` **SHOULD** be reserved for W3C trace context:

* `traceparent`
* `tracestate`
* `baggage`

Implementations **MUST NOT** add any custom fields at the root of a type that's part of the specification. All possible names are reserved for future protocol versions.

## Extension Methods

The protocol reserves any method name starting with an underscore (`_`) for custom extensions. This allows implementations to add new functionality without the risk of conflicting with future protocol versions.

Extension methods follow standard JSON-RPC 2.0 semantics.

### Custom Requests

Implementations **MAY** expose and call custom JSON-RPC requests as long as their name starts with an underscore (`_`).

If the receiving end doesn't recognize the custom method name, it should respond with the standard “Method not found” error.

To avoid such cases, extensions **SHOULD** advertise their custom capabilities so that callers can check their availability first and adapt their behavior or interface accordingly.

### Custom Notifications

Custom notifications are regular JSON-RPC notifications that start with an underscore (`_`).

Unlike with custom requests, implementations **SHOULD** ignore unrecognized notifications.

## Advertising Custom Capabilities

Implementations **SHOULD** use the `_meta` field in capability objects to advertise support for extensions and their methods.

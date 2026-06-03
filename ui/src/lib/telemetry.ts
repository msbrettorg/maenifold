// Telemetry shim.
//
// The previous implementation wrapped @microsoft/applicationinsights-web
// and shipped events to a hardcoded Azure AppInsights endpoint. That
// dependency was removed because:
//   1. Vite's Rolldown bundler emitted multiple [INVALID_ANNOTATION]
//      warnings against the AppInsights `/* #__PURE__ */` comment
//      positions (a third-party issue we don't control at the source).
//   2. Shipping client telemetry to a Microsoft-hosted endpoint by default
//      conflicts with the project's localhost-trust security model
//      (see AGENTS.md "MCP Boundary" and "Cargo-cult security" sections).
//
// The public API is preserved as no-ops so callers in App.vue and
// stores/session.ts don't need to change. To re-enable telemetry against
// a different backend, replace the no-op bodies below.

export async function initTelemetry(_enabled: boolean = true): Promise<void> {
  // no-op
}

export function trackEvent(_name: string, _properties?: Record<string, string>): void {
  // no-op
}

export function trackError(_error: Error, _properties?: Record<string, string>): void {
  // no-op
}

export function trackMetric(
  _name: string,
  _value: number,
  _properties?: Record<string, string>,
): void {
  // no-op
}

export function setTelemetryEnabled(_enabled: boolean): void {
  // no-op
}

export function isTelemetryEnabled(): boolean {
  return false;
}

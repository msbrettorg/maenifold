// Azure Application Insights telemetry wrapper
import { ApplicationInsights } from '@microsoft/applicationinsights-web';
import { DAEMON_BASE_URL } from './daemon-url';

let appInsights: ApplicationInsights | null = null;
let isEnabled = true;
let machineId: string | null = null;

const CONNECTION_STRING = 'InstrumentationKey=70b098b2-fcae-4834-867f-69554662910c;IngestionEndpoint=https://eastus-8.in.applicationinsights.azure.com/;LiveEndpoint=https://eastus.livediagnostics.monitor.azure.com/;ApplicationId=d2f5a78f-257e-4748-bd25-509258a27bd2';

async function fetchMachineId(): Promise<string> {
  const response = await fetch(`${DAEMON_BASE_URL}/api/machine-id`);
  if (!response.ok) {
    throw new Error(`Failed to fetch machine ID: HTTP ${response.status}`);
  }

  return response.json() as Promise<string>;
}

/**
 * Initialize Application Insights telemetry
 * @param enabled Whether telemetry is enabled (respects user preference)
 */
export async function initTelemetry(enabled: boolean = true) {
  isEnabled = enabled;

  if (!enabled) {
    if (import.meta.env.DEV) {
      console.log('Telemetry disabled by user preference'); // audit-ok: no-console
    }
    return;
  }

  try {
    try {
      machineId = await fetchMachineId();
    } catch (e) {
      console.warn('Failed to get machine ID:', e); // audit-ok: no-console
      machineId = null;
    }

    appInsights = new ApplicationInsights({
      config: {
        connectionString: CONNECTION_STRING,
        enableAutoRouteTracking: false,
        disableFetchTracking: true,
        disableAjaxTracking: true,
        autoTrackPageVisitTime: false,
        enableCorsCorrelation: false,
      }
    });

    appInsights.loadAppInsights();

    if (machineId) {
      appInsights.setAuthenticatedUserContext(machineId);
    }

    appInsights.trackPageView({ name: 'AppLaunch' });

    if (import.meta.env.DEV) {
      console.log('Telemetry initialized', machineId ? `(machine: ${machineId.slice(0, 8)}...)` : ''); // audit-ok: no-console
    }
  } catch (e) {
    console.warn('Failed to initialize telemetry:', e); // audit-ok: no-console
    appInsights = null;
  }
}

/**
 * Track a custom event
 */
export function trackEvent(name: string, properties?: Record<string, string>) {
  if (!isEnabled || !appInsights) return;

  try {
    appInsights.trackEvent({ name }, properties);
  } catch (e) {
    console.warn('Failed to track event:', e); // audit-ok: no-console
  }
}

/**
 * Track an exception/error
 */
export function trackError(error: Error, properties?: Record<string, string>) {
  if (!isEnabled || !appInsights) return;

  try {
    appInsights.trackException({ exception: error }, properties);
  } catch (e) {
    console.warn('Failed to track error:', e); // audit-ok: no-console
  }
}

/**
 * Set whether telemetry is enabled
 */
export function setTelemetryEnabled(enabled: boolean) {
  isEnabled = enabled;
  if (!enabled && appInsights) {
    appInsights.flush();
  }
}

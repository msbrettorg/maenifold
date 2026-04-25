export type SchedulePlatformSupport = {
  platform: 'macos' | 'linux' | 'windows' | 'unknown';
  label: string;
  scheduler: string;
  supportsEnabledSchedules: boolean;
  description: string;
  unsupportedEnabledMessage: string;
};

export function detectSchedulePlatform(): SchedulePlatformSupport {
  const userAgent = navigator.userAgent.toLowerCase();
  if (userAgent.includes('mac os') || userAgent.includes('macintosh')) {
    return {
      platform: 'macos',
      label: 'macOS supported',
      scheduler: 'launchd LaunchAgents',
      supportsEnabledSchedules: true,
      description: 'Enabled schedules install macOS launchd LaunchAgents.',
      unsupportedEnabledMessage: '',
    };
  }
  if (userAgent.includes('linux')) {
    return {
      platform: 'linux',
      label: 'Linux supported',
      scheduler: 'systemd user timers',
      supportsEnabledSchedules: true,
      description: 'Enabled schedules install Linux systemd user timers.',
      unsupportedEnabledMessage: '',
    };
  }
  if (userAgent.includes('windows')) {
    return {
      platform: 'windows',
      label: 'Windows unsupported',
      scheduler: 'not implemented',
      supportsEnabledSchedules: false,
      description: 'Enabled schedules are currently blocked because Windows scheduler integration is not implemented.',
      unsupportedEnabledMessage: 'Windows scheduler integration is not implemented. You can save schedules only while paused.',
    };
  }
  return {
    platform: 'unknown',
    label: 'Platform unsupported',
    scheduler: 'not implemented',
    supportsEnabledSchedules: false,
    description: 'Enabled schedules are available on macOS launchd and Linux systemd user timers only.',
    unsupportedEnabledMessage: 'Enabled schedules require macOS launchd or Linux systemd user timers. You can save schedules only while paused.',
  };
}

const FALLBACK_SIDEBAR_WIDTH = 320
const FALLBACK_SIDEBAR_MIN_WIDTH = 200
const FALLBACK_SIDEBAR_MAX_WIDTH = 500

function parsePixelValue(value: string, fallback: number) {
  const parsed = Number.parseFloat(value)
  return Number.isFinite(parsed) ? parsed : fallback
}

export function getSidebarSizingTokens() {
  if (typeof window === 'undefined') {
    return {
      width: FALLBACK_SIDEBAR_WIDTH,
      minWidth: FALLBACK_SIDEBAR_MIN_WIDTH,
      maxWidth: FALLBACK_SIDEBAR_MAX_WIDTH,
    }
  }

  const styles = getComputedStyle(document.documentElement)

  return {
    width: parsePixelValue(styles.getPropertyValue('--sidebar-width'), FALLBACK_SIDEBAR_WIDTH),
    minWidth: parsePixelValue(styles.getPropertyValue('--sidebar-min-width'), FALLBACK_SIDEBAR_MIN_WIDTH),
    maxWidth: parsePixelValue(styles.getPropertyValue('--sidebar-max-width'), FALLBACK_SIDEBAR_MAX_WIDTH),
  }
}

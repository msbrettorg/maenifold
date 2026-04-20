export const NODE_TYPE_COLORS: Record<string, string> = {
  workflow: 'var(--color-brand)',
  prompt: 'color-mix(in srgb, var(--color-brand) 55%, var(--text) 45%)',
  sequentialThinking: 'var(--plum)',
  tool: 'var(--sage)',
};

export const STATUS_COLORS: Record<string, string> = {
  active: 'var(--color-brand)',
  completed: 'var(--sage)',
  stuck: 'var(--terracotta)',
  abandoned: 'var(--text-secondary)',
};

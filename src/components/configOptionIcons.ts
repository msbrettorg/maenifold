function getModelIcon(modelId: string): string {
  const lower = modelId.toLowerCase();
  if (lower.includes('claude')) return '🟣';
  if (lower.includes('gpt') || lower.includes('openai')) return '🟢';
  if (lower.includes('gemini') || lower.includes('google')) return '🔵';
  if (lower.includes('llama') || lower.includes('meta')) return '🟠';
  if (lower.includes('mistral')) return '🔴';
  if (lower.includes('deepseek')) return '🟤';
  return '🤖';
}

function getModeIcon(modeId: string): string {
  switch (modeId.toLowerCase()) {
    case 'ask':
      return '🎯';
    case 'architect':
    case 'plan':
      return '📐';
    case 'code':
    case 'auto':
      return '💻';
    case 'debug':
      return '🐛';
    case 'review':
      return '👀';
    default:
      return '⚙️';
  }
}

export function getOptionIcon(value: string, category?: string): string {
  switch (category) {
    case 'model':
      return getModelIcon(value);
    case 'mode':
      return getModeIcon(value);
    case 'thought_level':
      return '🧠';
    default:
      return '⚙️';
  }
}

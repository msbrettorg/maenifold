const FRONTMATTER_PATTERN = /^---\s*\r?\n[\s\S]*?\r?\n---\s*(?:\r?\n|$)/;
const GATE_MANAGED_BULLETS = ['- Stage ID:', '- Record Kind:'] as const;

function stripLeadingFrontmatter(content: string): string {
  return content.replace(FRONTMATTER_PATTERN, '');
}

function isGateManagedSection(sectionBody: string): boolean {
  return GATE_MANAGED_BULLETS.every((bullet) => sectionBody.includes(bullet));
}

export function parseGateTaskBodyContent(content: string): string {
  const withoutFrontmatter = stripLeadingFrontmatter(content).trimStart();

  if (!withoutFrontmatter) {
    return '';
  }

  const sectionPattern = /^##\s+.+$/gm;
  const sectionMatches = [...withoutFrontmatter.matchAll(sectionPattern)];

  for (const [index, match] of sectionMatches.entries()) {
    const start = match.index ?? 0;
    const nextStart = sectionMatches[index + 1]?.index ?? withoutFrontmatter.length;
    const sectionBody = withoutFrontmatter.slice(start, nextStart);

    if (isGateManagedSection(sectionBody)) {
      return withoutFrontmatter.slice(0, start).trimEnd();
    }
  }

  return withoutFrontmatter.trimEnd();
}

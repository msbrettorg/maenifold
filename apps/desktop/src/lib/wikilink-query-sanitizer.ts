export function sanitizeConceptForWikilinkQuery(name: string): string {
  return name
    .replace(/[\[\]]/g, '')
    .replace(/[\r\n]+/g, ' ')
    .trim();
}

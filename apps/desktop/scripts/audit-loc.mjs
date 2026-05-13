export function countLinesForMaxLoc(source) {
  // Match `wc -l`: count newline bytes, not split() entries after a trailing newline.
  return (source.match(/\n/g) ?? []).length;
}

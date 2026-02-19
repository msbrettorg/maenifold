#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Manual builder for Maenifold release artifacts.

Usage:
  scripts/manual-release-build.sh <version> [output-dir]

  <version>     Semantic version number (e.g., 1.0.3 or v1.0.3).
  [output-dir]  Destination directory for archives (default: dist/releases).

The script produces platform archives + SHA256 sums for:
  - linux-x64 (.zip)
  - osx-arm64 (.zip)
  - osx-x64 (.zip)
  - win-x64 (.zip)

Archives are named maenifold-<version>-<rid>.zip with <version> stripped of any leading 'v'.
USAGE
}

if [[ ${1:-} == "" ]]; then
  usage
  exit 1
fi

RAW_VERSION="$1"
VERSION="${RAW_VERSION#v}"  # strip leading v if present
OUTPUT_ROOT=${2:-dist/releases}

PROJECT_ROOT=$(cd -- "$(dirname -- "$0")/.." && pwd)
cd "$PROJECT_ROOT"

if ! command -v dotnet >/dev/null 2>&1; then
  echo "dotnet CLI not found; install .NET SDK 9.0+" >&2
  exit 1
fi

if command -v shasum >/dev/null 2>&1; then
  HASH_CMD="shasum -a 256"
elif command -v sha256sum >/dev/null 2>&1; then
  HASH_CMD="sha256sum"
else
  echo "Neither shasum nor sha256sum available for SHA256 generation" >&2
  exit 1
fi

ARTIFACT_ROOT="${PROJECT_ROOT}/artifacts/manual-release"
rm -rf "$ARTIFACT_ROOT"
mkdir -p "$ARTIFACT_ROOT"

mkdir -p "$OUTPUT_ROOT"
OUTPUT_ROOT=$(cd -- "$OUTPUT_ROOT" && pwd)

RUNTIMES=(
  "linux-x64"
  "osx-arm64"
  "osx-x64"
  "win-x64"
)

for runtime in "${RUNTIMES[@]}"; do
  publish_dir="$ARTIFACT_ROOT/$runtime"
  rm -rf "$publish_dir"
  mkdir -p "$publish_dir"

  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "Building runtime: $runtime"
  echo "Output: $publish_dir"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

  dotnet publish src/Maenifold.csproj \
    -c Release \
    -r "$runtime" \
    --self-contained true \
    -p:PublishSingleFile=true \
    -o "$publish_dir"

  cp README.md "$publish_dir"/
  [[ -f LICENSE ]] && cp LICENSE "$publish_dir"/

  if [[ ! -d "$publish_dir/assets" ]]; then
    echo "ERROR: assets/ directory missing in publish output for $runtime" >&2
    exit 1
  fi

  if [[ "$runtime" == win-* ]]; then
    binary="$publish_dir/maenifold.exe"
  else
    binary="$publish_dir/maenifold"
  fi

  if [[ ! -f "$binary" ]]; then
    echo "ERROR: expected binary $binary was not generated" >&2
    exit 1
  fi

  archive_name="maenifold-${VERSION}-${runtime}"
  archive_path="$OUTPUT_ROOT/${archive_name}.zip"
  (cd "$publish_dir" && zip -qry "$archive_path" .)

  hash_path="${archive_path}.sha256"
  (cd "$OUTPUT_ROOT" && $HASH_CMD "$(basename "$archive_path")") > "$hash_path"

  echo "Created $(basename "$archive_path")"
  echo "SHA256 -> $(basename "$hash_path")"
done

cat <<SUMMARY

✅ Finished building manual release artifacts for version ${VERSION}
Files are located in: ${OUTPUT_ROOT}
SUMMARY

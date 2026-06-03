<#
.SYNOPSIS
    Evict stale per-agent Cargo build caches under target/agents/<task-id>.

.DESCRIPTION
    Windows port of scripts/clean-stale-agent-caches.sh. The TTL defaults to 7
    days and can be overridden with the MAENIFOLD_AGENT_CACHE_TTL_DAYS
    environment variable. The script is a dry run by default: it lists cache
    directories and the MB that would be reclaimed. Pass -Apply to actually
    delete stale caches.

.PARAMETER Apply
    Delete caches whose age is >= the TTL. Without this switch the script only
    reports what would be reclaimed.

.EXAMPLE
    pwsh scripts/clean-stale-agent-caches.ps1
    pwsh scripts/clean-stale-agent-caches.ps1 -Apply
#>
[CmdletBinding()]
param(
    [switch]$Apply
)

$ErrorActionPreference = 'Stop'

$ttlRaw = $env:MAENIFOLD_AGENT_CACHE_TTL_DAYS
if ([string]::IsNullOrWhiteSpace($ttlRaw)) {
    $ttlDays = 7
} elseif ($ttlRaw -match '^\d+$') {
    $ttlDays = [int]$ttlRaw
} else {
    Write-Error 'MAENIFOLD_AGENT_CACHE_TTL_DAYS must be a non-negative integer'
    exit 2
}

# Resolve repo root as the parent of this script's directory so the script works
# regardless of the current working directory.
$repoRoot = Split-Path -Parent $PSScriptRoot
$agentsDir = Join-Path $repoRoot 'target/agents'

if (-not (Test-Path -LiteralPath $agentsDir -PathType Container)) {
    Write-Host 'target/agents/ does not exist (no caches to evict)'
    exit 0
}

$now = Get-Date
$count = 0
$staleCount = 0
$reclaimedBytes = 0L

Get-ChildItem -LiteralPath $agentsDir -Directory | Sort-Object Name | ForEach-Object {
    $cache = $_
    $ageDays = [int][math]::Floor(($now - $cache.LastWriteTime).TotalDays)
    if ($ageDays -lt 0) { $ageDays = 0 }
    $sizeBytes = (Get-ChildItem -LiteralPath $cache.FullName -Recurse -File -ErrorAction SilentlyContinue |
        Measure-Object -Property Length -Sum).Sum
    if ($null -eq $sizeBytes) { $sizeBytes = 0L }

    '{0,4}d  {1,8:N1} MB  {2}' -f $ageDays, ($sizeBytes / 1MB), $cache.Name | Write-Host
    $script:count++

    if ($ageDays -lt $ttlDays) { return }

    $script:staleCount++
    $script:reclaimedBytes += $sizeBytes

    if ($Apply) {
        Remove-Item -LiteralPath $cache.FullName -Recurse -Force -ErrorAction SilentlyContinue
    }
}

$reclaimedMb = $reclaimedBytes / 1MB
if ($Apply) {
    Write-Host ('Reclaimed: {0:N1} MB from {1} caches (TTL {2}d)' -f $reclaimedMb, $staleCount, $ttlDays)
} else {
    Write-Host ('Total: {0} caches, {1:N1} MB would be reclaimed (TTL {2}d, run with -Apply to delete)' -f $count, $reclaimedMb, $ttlDays)
}

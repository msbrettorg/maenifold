<#
.SYNOPSIS
    Windows release rebuild + daemon restart for maenifold (parity with rebuild.sh).

.DESCRIPTION
    The macOS rebuild.sh relies on launchd/symlinks which do not exist on Windows.
    This script provides the equivalent Windows flow:

      1. Evict stale per-agent build caches (target/agents) so disk cannot grow
         unbounded — the cleanup the bash flow never had on Windows.
      2. Regenerate the tools manifest.
      3. Build every crate in release into the canonical target/release.
      4. Stop the running daemon and start the freshly built binary (--http).
      5. Verify the MCP endpoint and the maenifold.gate domain registration.

.PARAMETER Port
    HTTP port for the daemon. Default 8080.

.EXAMPLE
    pwsh rebuild.ps1
#>
[CmdletBinding()]
param(
    [int]$Port = 8080
)

$ErrorActionPreference = 'Stop'
$repoRoot = $PSScriptRoot
Set-Location $repoRoot

# Approved-build token. The maenifold-core/gate-core/jockey build scripts reject
# any compile that does not carry this token (see AGENTS.md > Build). Setting it
# here is what makes the rebuild script the only approved build path.
$env:MAENIFOLD_REBUILD = '1'

# Interactive/canonical build must use the workspace target, never an agent cache.
Remove-Item Env:\CARGO_TARGET_DIR -ErrorAction SilentlyContinue

Write-Host '=== Evict stale agent build caches ==='
& (Join-Path $repoRoot 'scripts/clean-stale-agent-caches.ps1') -Apply

Write-Host ''
Write-Host '=== Regenerate tools manifest ==='
cargo run --quiet --bin generate-tools-manifest
if ($LASTEXITCODE -ne 0) { Write-Error 'manifest generation failed'; exit 1 }

Write-Host ''
Write-Host '=== Stop running daemon (release the exe + port) ==='
$listener = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue
if ($listener) {
    $daemonPid = ($listener.OwningProcess | Select-Object -First 1)
    try {
        Stop-Process -Id $daemonPid -Force -ErrorAction Stop
        Write-Host "  stopped daemon PID $daemonPid"
        Start-Sleep -Seconds 2
    } catch {
        Write-Host "  could not stop PID ${daemonPid}: $($_.Exception.Message)"
    }
} else {
    Write-Host "  nothing listening on port $Port"
}

Write-Host ''
Write-Host '=== Release build: all crates ==='
cargo build --release -p maenifold-jockey -p maenifold-core -p maenifold-pipeline-driver -p maenifold-cli
if ($LASTEXITCODE -ne 0) { Write-Error 'release build failed'; exit 1 }

$exe = Join-Path $repoRoot 'target/release/maenifold.exe'
if (-not (Test-Path -LiteralPath $exe)) { Write-Error "binary missing at $exe"; exit 1 }

Write-Host ''
Write-Host '=== Start daemon ==='
$proc = Start-Process -FilePath $exe -ArgumentList '--http', '--port', "$Port" -PassThru -WindowStyle Hidden
Write-Host "  started daemon PID $($proc.Id) ($exe --http --port $Port)"

Write-Host ''
Write-Host '=== Verify MCP endpoint ==='
$verifyFailed = 0
$mcpOk = $false
for ($i = 0; $i -lt 20; $i++) {
    Start-Sleep -Seconds 1
    try {
        Invoke-WebRequest -Uri "http://127.0.0.1:$Port/mcp" -Method Get -TimeoutSec 5 -UseBasicParsing | Out-Null
    } catch {
        $code = $_.Exception.Response.StatusCode.value__
        if ($code -eq 405 -or $code -eq 406) { $mcpOk = $true; break }
    }
}
if ($mcpOk) {
    Write-Host "  OK localhost:$Port/mcp responding"
} else {
    Write-Host "  FAIL localhost:$Port/mcp not responding"
    $verifyFailed = 1
}

Write-Host ''
Write-Host '=== Verify maenifold.gate domain registration ==='
$gateOk = $false
$headers = @{ 'Content-Type' = 'application/json'; 'Accept' = 'application/json, text/event-stream' }
$body = '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"GatewayIndex","arguments":{}}}'
for ($i = 0; $i -lt 15; $i++) {
    try {
        $resp = Invoke-RestMethod -Uri "http://127.0.0.1:$Port/mcp" -Method Post -Headers $headers -Body $body -TimeoutSec 10
        if (($resp | ConvertTo-Json -Depth 10) -match 'maenifold\.gate') { $gateOk = $true; break }
    } catch {}
    Start-Sleep -Seconds 1
}
if ($gateOk) {
    Write-Host '  OK maenifold.gate domain registered'
} else {
    Write-Host '  FAIL maenifold.gate domain missing (daemon may be running a stale binary)'
    $verifyFailed = 1
}

Write-Host ''
Write-Host 'Done.'
exit $verifyFailed

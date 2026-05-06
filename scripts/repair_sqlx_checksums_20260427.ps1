#Requires -Version 5.1
<#
  Runs repair_sqlx_checksums_20260427.sql without requiring psql on PATH.
  Tries, in order:
  1) psql on PATH
  2) PostgreSQL installation under "C:\Program Files\PostgreSQL\*\bin\psql.exe"
  3) Docker: postgres:16-alpine + psql (use host.docker.internal to reach DB on the host)
#>
$ErrorActionPreference = "Stop"

# Load .env from backend root (parent of scripts/) if DATABASE_URL is unset
if (-not $env:DATABASE_URL) {
  $envFile = Join-Path (Split-Path $PSScriptRoot -Parent) ".env"
  if (Test-Path $envFile) {
    Get-Content $envFile | ForEach-Object {
      if ($_ -match '^\s*DATABASE_URL=(.+)\s*$') { $env:DATABASE_URL = $matches[1].Trim('"', "'") }
    }
  }
}
if (-not $env:DATABASE_URL) {
  Write-Error "Set DATABASE_URL, e.g.:
  `$env:DATABASE_URL = 'postgres://user:pass@localhost:5432/wowonder'
  Or add DATABASE_URL=... to backend\.env"
}

$sqlFile = Join-Path $PSScriptRoot "repair_sqlx_checksums_20260427.sql"
if (-not (Test-Path $sqlFile)) {
  Write-Error "Missing file: $sqlFile"
}

function Invoke-Psql {
  param([string] $PsqlPath, [string] $Url)
  & $PsqlPath $Url -f $sqlFile
  return $LASTEXITCODE
}

# 1) PATH
$psql = (Get-Command psql -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty Source)

# 2) Standard Windows install
if (-not $psql) {
  $found = Get-ChildItem "C:\Program Files\PostgreSQL" -Recurse -Filter "psql.exe" -ErrorAction SilentlyContinue |
    Select-Object -First 1 -ExpandProperty FullName
  if ($found) { $psql = $found }
}

if ($psql) {
  Write-Host "Using: $psql" -ForegroundColor Cyan
  $code = Invoke-Psql -PsqlPath $psql -Url $env:DATABASE_URL
  if ($code -ne 0) { exit $code }
  Write-Host "OK. Now run: sqlx migrate run" -ForegroundColor Green
  exit 0
}

# 3) Docker: DB must be reachable from the container (host machine)
$docker = Get-Command docker -ErrorAction SilentlyContinue
if ($docker) {
  $url = $env:DATABASE_URL
  if ($url -match '@localhost:') { $url = $url -replace '@localhost:', '@host.docker.internal:' }
  if ($url -match '@127\.0\.0\.1:') { $url = $url -replace '@127\.0\.0\.1:', '@host.docker.internal:' }

  $dir = (Resolve-Path (Split-Path $sqlFile -Parent)).Path
  $name = Split-Path $sqlFile -Leaf

  Write-Host "Using Docker (postgres:16-alpine) + psql. Host DB must be reachable as host.docker.internal (Docker Desktop on Windows)." -ForegroundColor Cyan
  $args = @(
    "run", "--rm",
    "-v", "${dir}:/mig:ro",
    "postgres:16-alpine",
    "psql", $url, "-f", "/mig/$name"
  )
  & docker @args
  if ($LASTEXITCODE -ne 0) {
    Write-Host "If the DB runs on this PC, check Docker Desktop and that the URL uses host.docker.internal above." -ForegroundColor Yellow
    exit $LASTEXITCODE
  }
  Write-Host "OK. Now run: sqlx migrate run" -ForegroundColor Green
  exit 0
}

Write-Error @"
Could not find psql or docker.

Option A — Install client tools: winget install PostgreSQL.PostgreSQL.16
  (or add the existing bin e.g. C:\Program Files\PostgreSQL\16\bin to PATH), then re-run this script.

Option B — Open PgAdmin / DBeaver / Azure Data Studio (PostgreSQL) and execute the SQL from:
  $sqlFile

Option C — From WSL, if you have psql there:
  wsl -e psql ``$env:DATABASE_URL`` -f (wslpath path)

"@

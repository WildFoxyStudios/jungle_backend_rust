# Deploy Jungle API Gateway to Fly.io
# Builds the Docker image locally and pushes to Fly.io's registry.
# Requires: Docker Desktop running, fly CLI authenticated (fly auth login).

Set-Location $PSScriptRoot

Write-Host "Deploying jungle-api-gw to Fly.io (local build)..." -ForegroundColor Cyan
fly deploy --local-only -a jungle-api-gw

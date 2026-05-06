#!/bin/bash
# Deploy Jungle API Gateway to Fly.io
# Builds the Docker image locally and pushes to Fly.io's registry.
# Requires: Docker running, fly CLI authenticated (fly auth login).

set -e
cd "$(dirname "$0")"
echo "Deploying jungle-api-gw to Fly.io (local build)..."
fly deploy --local-only -a jungle-api-gw

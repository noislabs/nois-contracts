#!/bin/bash
set -o errexit -o nounset -o pipefail
command -v shellcheck >/dev/null && shellcheck "$0"

# Usage:
# 1. Make sure you can login to your remote server without password on the default port (22)
# 2. Run `REMOTE=root@116.203.108.115 ./ci-scripts/remote_start.sh` from the repo root with your user and host

echo "REMOTE = $REMOTE"

echo "Testing ssh connection ..."
ssh "$REMOTE" "date"

echo "Stopping chains …"
ssh -t "$REMOTE" "bash -c './ci-scripts/nois/stop.sh'"
ssh -t "$REMOTE" "bash -c './ci-scripts/wasmd/stop.sh'"

sleep 1

echo "Docker processes left …"
ssh -t "$REMOTE" "docker ps"

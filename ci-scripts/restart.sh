#!/bin/bash
set -o errexit -o nounset -o pipefail
command -v shellcheck >/dev/null && shellcheck "$0"

SCRIPT_DIR="$(realpath "$(dirname "$0")")"

docker kill nois || true
docker kill wasmd || true

# Ensure all volumes are freed
sleep 1

"$SCRIPT_DIR"/nois/start.sh > /dev/null &
"$SCRIPT_DIR"/wasmd/start.sh > /dev/null &

# Wait a bit for things to start and log processes
sleep 2
docker ps

#!/bin/bash
set -o errexit -o nounset -o pipefail
command -v shellcheck >/dev/null && shellcheck "$0"

SCRIPT_DIR="$(realpath "$(dirname "$0")")"

docker kill noisd-ci || true
docker kill osmosis || true
docker kill wasmd || true

# Ensure all volumes are freed
sleep 1

"$SCRIPT_DIR"/nois/start.sh > /dev/null &
"$SCRIPT_DIR"/osmosis/start.sh > /dev/null &
"$SCRIPT_DIR"/wasmd/start.sh > /dev/null &

watch docker ps

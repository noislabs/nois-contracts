#!/bin/bash
set -o errexit -o nounset -o pipefail
command -v shellcheck >/dev/null && shellcheck "$0"

# Please keep this in sync with the Ports overview in HACKING.md
TENDERMINT_PORT_GUEST="26657"
TENDERMINT_PORT_HOST="26655"

SCRIPT_DIR="$(realpath "$(dirname "$0")")"
# shellcheck source=./env
# shellcheck disable=SC1091
source "$SCRIPT_DIR"/env

# TMP_DIR=$(mktemp -d "${TMPDIR:-/tmp}/wasmd.XXXXXXXXX")
# chmod 777 "$TMP_DIR"
# echo "Using temporary dir $TMP_DIR"
# WASMD_LOGFILE="$TMP_DIR/wasmd.log"

# Use a fresh volume for every start
docker volume rm -f noisd-data
# only pull if we don't have it
(docker images | grep "$REPOSITORY" | grep -q "$VERSION") || docker pull "$REPOSITORY:$VERSION"

# This starts up noisd
echo "starting noisd with rpc on port $TENDERMINT_PORT_HOST"
docker run --rm \
  --name "$CONTAINER_NAME" \
  -p "$TENDERMINT_PORT_HOST":"$TENDERMINT_PORT_GUEST" \
  --mount type=bind,source="$SCRIPT_DIR/template",target=/template \
  --mount type=volume,source=noisd-data,target=/root \
  "$REPOSITORY:$VERSION" \
  run_ci.sh \
  2>&1 | tee debug-noisd.log | grep 'executed block'

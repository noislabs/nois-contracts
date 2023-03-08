#!/bin/bash
set -o errexit -o nounset -o pipefail
command -v shellcheck >/dev/null && shellcheck "$0"

SCRIPT_DIR="$(realpath "$(dirname "$0")")"
# shellcheck source=./env
# shellcheck disable=SC1091
source "$SCRIPT_DIR"/env

mkdir -p "$SCRIPT_DIR"/template

export CHAIN_ID=noisd-1

# The usage of the accounts below is documented in README.md of this directory
docker run --rm \
  -e PASSWORD=my-secret-password \
  -e CHAIN_ID \
  --mount type=bind,source="$SCRIPT_DIR/template",target=/root \
  "$REPOSITORY:$VERSION" \
  /opt/setup.sh \
  nois1hqg5nqnka82cwm3v02xj6ufns9tmss7rlpvucx

# chmod -R g+rwx template/.noisd/
# chmod -R a+rx template/.noisd/

# The ./template folder is created by the docker daemon's user (root on Linux, current user
# when using Docker Desktop on macOS), let's make it ours if needed
if [ ! -x "$SCRIPT_DIR/template/.noisd/config/gentx" ]; then
  sudo chown -R "$(id -u):$(id -g)" "$SCRIPT_DIR/template"
fi

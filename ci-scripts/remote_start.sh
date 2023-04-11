#!/bin/bash
set -o errexit -o nounset -o pipefail
command -v shellcheck >/dev/null && shellcheck "$0"

# Usage:
# 1. Make sure you can login to your remote server without password on the default port (22)
# 2. Run `REMOTE=root@116.203.108.115 ./ci-scripts/remote_start.sh` from the repo root with your user and host

echo "REMOTE = $REMOTE"

echo "Testing ssh connection ..."
ssh "$REMOTE" "date"

echo "Testing scp connection ..."
echo "Copy file test OK." > a_local_file.txt
scp a_local_file.txt "$REMOTE:~/a_remote_file.txt"
rm a_local_file.txt
ssh "$REMOTE" "cat ~/a_remote_file.txt"

echo "Updating packages …"
ssh -t "$REMOTE" "apt update -y && apt upgrade -y"

echo "Installing packages …"
ssh -t "$REMOTE" "apt install -y jq net-tools git docker.io cowsay"

echo "Copying scripts folder …"
ssh -t "$REMOTE" "mkdir -p ~/ci-scripts"
# scp -r ./ci-scripts/* "$REMOTE:~/ci-scripts"
rsync -a ./ci-scripts/ "$REMOTE:~/ci-scripts"
ssh "$REMOTE" "ls -lA ~/ci-scripts"

echo "Starting/restarting chains …"
# shellcheck disable=SC2088
ssh -t "$REMOTE" "~/ci-scripts/restart.sh"

echo "Start port forwarding …"
# Local Port Forwarding, see https://help.ubuntu.com/community/SSH/OpenSSH/PortForwarding#Local_Port_Forwarding
PORT_NOIS=26655
PORT_WASMD=26659
ssh -L "$PORT_NOIS:localhost:$PORT_NOIS" -L "$PORT_WASMD:localhost:$PORT_WASMD" \
    "$REMOTE" \
    -t "cowsay 'Port forwarding enabled to localhost:$PORT_NOIS and localhost:$PORT_WASMD on you local machine. Keep this shell session alive. Use the logout command or Ctrl+D, Ctrl+C to shut down the connection.' && bash -l"

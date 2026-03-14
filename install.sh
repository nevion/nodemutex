#!/bin/bash
set -euo pipefail

echo "Building nodemutex (release)..."
cargo build --release --manifest-path="$(dirname "$0")/Cargo.toml"

echo "Installing binary to /usr/local/bin/nodemutex..."
sudo install -m 0755 "$(dirname "$0")/target/release/nodemutex" /usr/local/bin/nodemutex

echo "Installing systemd unit..."
sudo install -m 0644 "$(dirname "$0")/nodemutex.service" /etc/systemd/system/nodemutex.service

echo "Reloading systemd and enabling service..."
sudo systemctl daemon-reload
sudo systemctl enable --now nodemutex.service

echo "Done. Status:"
systemctl status nodemutex.service --no-pager

#!/usr/bin/env bash
set -euo pipefail

rustup target add x86_64-unknown-linux-musl

echo "=== Building client package (musl target) ==="
cargo build -p client --release --target x86_64-unknown-linux-musl

CLIENT_BIN="target/x86_64-unknown-linux-musl/release/client"
DEST_DIR="server/assets"
mkdir -p "$DEST_DIR"
cp "$CLIENT_BIN" "$DEST_DIR/"

echo "✅ Copied client binary to $DEST_DIR/"

echo "=== Building server package ==="
cargo build -p server --release

echo "=== Running server ==="
SERVER_BIN="target/release/server"
"$SERVER_BIN"

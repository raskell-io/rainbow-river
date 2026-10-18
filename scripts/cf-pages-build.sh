#!/usr/bin/env bash
# Build command for the Cloudflare Pages project (Git integration):
#   scripts/cf-pages-build.sh        output directory: dist
# Installs Rust with the wasm target when missing, fetches the WASM-AD player
# that ships with the site, then builds the site.
set -euo pipefail

if ! command -v cargo >/dev/null; then
    curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal
    export PATH="$HOME/.cargo/bin:$PATH"
fi
rustup target add wasm32-unknown-unknown

other=$(mktemp -d)/wasm-ad
git clone --depth 1 https://github.com/raskell-io/wasm-ad.git "$other"
make site WASM_AD="$other"

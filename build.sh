#!/usr/bin/env bash
# Builds the frontend and a static Linux server binary on the host, ready for
# the Dockerfile to copy. Works on macOS and Linux.
#   ./build.sh                    # host architecture
#   ARCH=amd64 ./build.sh         # one architecture
#   ARCH="amd64 arm64" ./build.sh # several
# Needs: rustup, zig, cargo-zigbuild, wasm-pack, npm.
set -euo pipefail
cd "$(dirname "$0")"

case "$(uname -m)" in
  arm64 | aarch64) host_arch=arm64 ;;
  x86_64 | amd64) host_arch=amd64 ;;
  *) echo "Unsupported host architecture: $(uname -m)" >&2; exit 1 ;;
esac
arches="${ARCH:-$host_arch}"

wasm-pack build rustpad-wasm
[ -d node_modules ] || npm ci
npm run check
VITE_SHA="${GITHUB_SHA:-$(git rev-parse HEAD)}" npm run build

cargo test -p rustpad-server --release

for arch in $arches; do
  case "$arch" in
    arm64) target=aarch64-unknown-linux-musl ;;
    amd64) target=x86_64-unknown-linux-musl ;;
    *) echo "Unsupported ARCH: $arch (use amd64 or arm64)" >&2; exit 1 ;;
  esac
  rustup target add "$target"
  cargo zigbuild --release -p rustpad-server --target "$target"
  mkdir -p "build/linux-$arch"
  cp "target/$target/release/rustpad-server" "build/linux-$arch/"
done

#!/bin/sh
# Runs the smoke tests in rustpad-server/tests/smoke.rs against a running
# go-judge runner (default http://127.0.0.1:5050).
set -eu
cd "$(dirname "$0")/.."
export GO_JUDGE_URL="${GO_JUDGE_URL:-http://127.0.0.1:5050}"
export RUST_LOG="${RUST_LOG:-rustpad_server=debug}"
exec cargo test -p rustpad-server --release --test smoke -- --ignored --nocapture --test-threads=1

#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
npm run typecheck
npm test -- --run
npm run build
node scripts/audit-js-offline.mjs --self-test
python3 scripts/test-recover-csv.py
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo build -p patrimoine-desktop --features custom-protocol --locked

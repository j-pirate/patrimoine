#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
npm run typecheck
npm test -- --run
npm run build
node scripts/audit-js-offline.mjs --self-test
python3 scripts/test-recover-csv.py
python3 scripts/test-qualify-performance.py
python3 scripts/test-notices.py
python3 scripts/test-source-identity.py
python3 scripts/test-build-linux.py
python3 scripts/test-advisory-report.py
python3 scripts/check-contracts.py
python3 scripts/test-check-contracts.py
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo build -p patrimoine-desktop --features custom-protocol --locked

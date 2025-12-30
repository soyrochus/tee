#!/usr/bin/env bash
set -euo pipefail

# Pass all script arguments to the cargo run command
cargo run --bin dbuserctl -- "$@"


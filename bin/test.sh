#!/usr/bin/env bash
# Run both workspaces even when one suite fails, preserving Cargo's exit status.
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

suite=${1:-all}
case "$suite" in
    all|client|server|database) ;;
    *)
        echo "Usage: $0 [all|client|server|database] [cargo test options...]" >&2
        exit 2
        ;;
esac
if [[ $# -gt 0 ]]; then
    shift
fi

mkdir -p target/test-results
export CARGO_TERM_COLOR=never
export RUST_BACKTRACE=${RUST_BACKTRACE:-1}
result=0

run_suite() {
    local name=$1
    shift
    echo "Running $name tests (log: target/test-results/$name.log)"
    if cargo test --locked "$@" 2>&1 | tee "target/test-results/$name.log"; then
        echo "$name: passed"
    else
        local status=$?
        echo "$name: failed (exit $status)" >&2
        result=$status
    fi
}

case "$suite" in
    all)
        run_suite client --package client --all-targets --no-fail-fast "$@"
        run_suite server --package server --all-targets --no-fail-fast "$@"
        ;;
    client|server)
        run_suite "$suite" --package "$suite" --all-targets --no-fail-fast "$@"
        ;;
    database)
        run_suite database --package server --features db-tests --bin server \
            tests::database:: "$@"
        ;;
esac

exit "$result"

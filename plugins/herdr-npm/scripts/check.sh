#!/bin/sh
set -eu

PLUGIN_DIR=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
REPO_DIR=$(CDPATH= cd -- "$PLUGIN_DIR/../.." && pwd)
MANIFEST="$PLUGIN_DIR/Cargo.toml"

cd "$REPO_DIR"

run_features() {
  tags=$1
  echo "== Cucumber ${tags:-all} =="
  if [ -n "$tags" ]; then
    cargo test --manifest-path "$MANIFEST" --test features -- --tags "$tags"
  else
    cargo test --manifest-path "$MANIFEST" --test features
  fi
}

mode=${1:-}
case "$mode" in
  harness)
    run_features ""
    ;;
  toggle|catalog|run)
    run_features "@$mode"
    ;;
  all)
    echo "== fmt =="
    cargo fmt --manifest-path "$MANIFEST" --check
    echo "== clippy =="
    cargo clippy --manifest-path "$MANIFEST" --all-targets -- -D warnings
    echo "== Rust + Cucumber =="
    cargo test --manifest-path "$MANIFEST"
    ;;
  *)
    echo "usage: $0 <harness|toggle|catalog|run|all>" >&2
    exit 2
    ;;
esac

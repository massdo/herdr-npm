#!/bin/sh
set -eu

PLUGIN_DIR=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
REPO_DIR=$(CDPATH= cd -- "$PLUGIN_DIR/../.." && pwd)
MANIFEST="$PLUGIN_DIR/Cargo.toml"
CARGO_TEST="cargo test --manifest-path $MANIFEST"

cd "$REPO_DIR"

count_features() {
  python3 - "$PLUGIN_DIR/tests/features" <<'PY'
from pathlib import Path
import sys
root = Path(sys.argv[1])
decl = cases = e2e = 0
print("discovered features:")
for path in sorted(root.glob("*.feature")):
    text = path.read_text()
    file_decl = file_cases = file_e2e = 0
    pending = []
    i = 0
    lines = text.splitlines()
    while i < len(lines):
        s = lines[i].strip()
        if s.startswith("@"):
            pending.extend(s.split())
            i += 1
            continue
        if s.startswith("Scenario Outline:") or s.startswith("Scenario:"):
            kind = "outline" if s.startswith("Scenario Outline:") else "scenario"
            tags = list(pending)
            pending = []
            n = 1
            if kind == "outline":
                n = 0
                j = i + 1
                in_ex = False
                header = False
                while j < len(lines):
                    t = lines[j].strip()
                    if t.startswith("@") or t.startswith("Scenario:") or t.startswith("Scenario Outline:"):
                        break
                    if t.startswith("Examples"):
                        in_ex = True
                        header = False
                        j += 1
                        continue
                    if in_ex and t.startswith("|"):
                        if not header:
                            header = True
                        else:
                            n += 1
                    j += 1
            file_decl += 1
            file_cases += n
            if "@e2e" in tags:
                file_e2e += n
        elif s.startswith("Feature:"):
            pending = []
        i += 1
    print(f"  {path.name}: {file_decl} declarations, {file_cases} cases ({file_e2e} e2e)")
    decl += file_decl
    cases += file_cases
    e2e += file_e2e
print(f"total: {decl} declarations, {cases} cases, {e2e} e2e, {cases - e2e} selected by default (not @e2e)")
PY
}

run_features() {
  tags=$1
  echo "+ $CARGO_TEST --test features -- --tags $tags"
  $CARGO_TEST --test features -- --tags "$tags"
}

mode=${1:-}
case "$mode" in
  harness)
    echo "== compile features target =="
    $CARGO_TEST --test features --no-run
    echo "== discover =="
    count_features
    echo "== business suite (expected red until later lots) =="
    out=$(mktemp)
    trap 'rm -f "$out"' EXIT
    set +e
    run_features "not @e2e" >"$out" 2>&1
    biz=$?
    set -e
    cat "$out"
    echo "business_suite_exit=$biz"
    if ! grep -E '^[0-9]+ scenarios' "$out" >/dev/null; then
      echo "harness: cucumber produced no scenario summary"
      exit 1
    fi
    summary=$(grep -E '^[0-9]+ scenarios' "$out" | tail -n 1)
    echo "business_suite_summary=$summary"
    case "$summary" in
      110\ scenarios*)
        ;;
      *)
        echo "harness: expected 110 scenarios after not @e2e (74 declarations expanded, 7 e2e excluded)"
        exit 1
        ;;
    esac
    if [ "$biz" -eq 0 ]; then
      echo "harness: compile+parse ok; business suite is green"
      exit 0
    fi
    # cucumber-rs panics (cargo 101) or we exit 1 from the features binary.
    if [ "$biz" -eq 1 ] || [ "$biz" -eq 101 ]; then
      echo "harness: compile+parse ok; business suite is red (exit $biz) as expected before toggle/catalog/run lots"
      exit 0
    fi
    echo "harness: cucumber did not run a business suite (exit $biz)"
    exit "$biz"
    ;;
  toggle)
    run_features "@toggle and not @e2e"
    ;;
  catalog)
    run_features "@catalog and not @e2e"
    ;;
  run)
    run_features "@run and not @e2e"
    ;;
  all)
    echo "== fmt =="
    cargo fmt --manifest-path "$MANIFEST" --check
    echo "== clippy =="
    cargo clippy --manifest-path "$MANIFEST" --all-targets -- -D warnings
    echo "== unit =="
    cargo test --manifest-path "$MANIFEST" --lib
    echo "== architecture =="
    cargo test --manifest-path "$MANIFEST" --test architecture
    echo "== features not @e2e =="
    run_features "not @e2e"
    ;;
  *)
    echo "usage: $0 <harness|toggle|catalog|run|all>" >&2
    exit 2
    ;;
esac

#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'USAGE'
Usage:
  scripts/compare-backend-compile-time.sh [-n RUNS] [--warmups RUNS] SOURCE [-- PLANK_BUILD_ARGS...]

Measures Plank build time for:
  sir-o0, sir-csud, sona-o0, sona-o2

The script builds the release CLI once, then times target/release/plank build.
Extra arguments after -- are passed to plank build before the backend flags.

Environment:
  PLANK_BIN=/path/to/plank   Use an existing plank binary instead of cargo build.
  RUNS=5                     Timed runs per backend.
  WARMUPS=1                  Untimed warmup runs per backend.

Examples:
  scripts/compare-backend-compile-time.sh plank-diff-tests/src/examples/erc20.plk
  scripts/compare-backend-compile-time.sh -n 10 contract.plk -- --dep foo=vendor/foo
USAGE
}

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"
runs="${RUNS:-5}"
warmups="${WARMUPS:-1}"
source_file=""
extra_args=()

while (($#)); do
    case "$1" in
        -h|--help)
            usage
            exit 0
            ;;
        -n|--runs)
            if (($# < 2)); then
                echo "error: $1 requires a value" >&2
                exit 2
            fi
            runs="$2"
            shift 2
            ;;
        --warmups)
            if (($# < 2)); then
                echo "error: $1 requires a value" >&2
                exit 2
            fi
            warmups="$2"
            shift 2
            ;;
        --)
            shift
            extra_args=("$@")
            break
            ;;
        -*)
            echo "error: unknown option $1" >&2
            usage >&2
            exit 2
            ;;
        *)
            if [[ -n "$source_file" ]]; then
                echo "error: unexpected argument $1; pass plank build args after --" >&2
                exit 2
            fi
            source_file="$1"
            shift
            ;;
    esac
done

if [[ -z "$source_file" ]]; then
    usage >&2
    exit 2
fi

if ! [[ "$runs" =~ ^[0-9]+$ ]] || ((runs < 1)); then
    echo "error: RUNS must be a positive integer" >&2
    exit 2
fi

if ! [[ "$warmups" =~ ^[0-9]+$ ]]; then
    echo "error: WARMUPS must be a non-negative integer" >&2
    exit 2
fi

if [[ ! -f "$source_file" ]]; then
    echo "error: source file not found: $source_file" >&2
    exit 1
fi

source_dir="$(cd "$(dirname "$source_file")" && pwd)"
source_file="$source_dir/$(basename "$source_file")"

if [[ -n "${PLANK_BIN:-}" ]]; then
    plank_bin="$PLANK_BIN"
else
    if ! build_output="$(cargo build --quiet --release -p plank 2>&1)"; then
        echo "$build_output" >&2
        exit 1
    fi
    plank_bin="$repo_root/target/release/plank"
fi

if [[ ! -x "$plank_bin" ]]; then
    echo "error: plank binary is not executable: $plank_bin" >&2
    exit 1
fi

base_args=(build "$source_file")
if ((${#extra_args[@]})); then
    base_args+=("${extra_args[@]}")
fi
std_root=""
for candidate in "$repo_root/std" "$repo_root/../std"; do
    if [[ -d "$candidate" ]]; then
        std_root="$(cd "$candidate" && pwd)"
        break
    fi
done
if [[ -n "$std_root" ]]; then
    base_args+=(--dep "std=$std_root")
fi

time_build() {
    local backend="$1"
    local optimize="$2"
    local cmd=("$plank_bin" "${base_args[@]}" --backend "$backend")

    if [[ -n "$optimize" ]]; then
        cmd+=(-O "$optimize")
    fi

    local elapsed
    if ! elapsed="$({ TIMEFORMAT='%3R'; time "${cmd[@]}" >/dev/null; } 2>&1)"; then
        echo "error: build failed for backend=$backend optimize=${optimize:-none}" >&2
        echo "$elapsed" >&2
        return 1
    fi

    echo "$elapsed" | tail -n 1
}

run_case() {
    local label="$1"
    local backend="$2"
    local optimize="${3:-}"
    local times=()
    local elapsed

    for ((i = 1; i <= warmups; i++)); do
        time_build "$backend" "$optimize" >/dev/null
    done

    for ((i = 1; i <= runs; i++)); do
        elapsed="$(time_build "$backend" "$optimize")"
        times+=("$elapsed")
    done

    awk -v label="$label" -v runs="$runs" -v values="${times[*]}" '
        BEGIN {
            split(values, times, " ")
            min = times[1] + 0
            max = times[1] + 0
            sum = 0
            for (i = 1; i <= runs; i++) {
                value = times[i] + 0
                sum += value
                if (value < min) min = value
                if (value > max) max = value
            }
            printf "%-10s %8.3f %8.3f %8.3f\n", label, sum / runs, min, max
        }
    '
}

echo "source: $source_file"
echo "runs: $runs timed, $warmups warmup"
echo
printf "%-10s %8s %8s %8s\n" "case" "avg(s)" "min(s)" "max(s)"
printf "%-10s %8s %8s %8s\n" "----" "------" "------" "------"
run_case "sir-o0" "sir"
run_case "sir-csud" "sir" "csud"
run_case "sona-o0" "sona"
run_case "sona-o2" "sona" "csud"

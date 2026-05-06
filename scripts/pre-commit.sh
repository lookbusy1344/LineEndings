#!/usr/bin/env bash
# pre-commit.sh — run this project's required checks before committing.
#
# Install (one-time setup):
#   ln -sf ../../scripts/pre-commit.sh .git/hooks/pre-commit
#
# Can also be run directly: ./scripts/pre-commit.sh

set -euo pipefail

readonly NEXTEST_TIMEOUT_SECONDS=300

resolve_script_path() {
    local source_path="$1"

    while [[ -L "${source_path}" ]]; do
        local source_dir
        source_dir="$(cd -P "$(dirname "${source_path}")" && pwd)"
        source_path="$(readlink "${source_path}")"

        if [[ "${source_path}" != /* ]]; then
            source_path="${source_dir}/${source_path}"
        fi
    done

    local resolved_dir
    resolved_dir="$(cd -P "$(dirname "${source_path}")" && pwd)"
    printf '%s/%s\n' "${resolved_dir}" "$(basename "${source_path}")"
}

run() {
    echo "==> $*"
    "$@"
}

is_rust_commit_relevant_path() {
    local staged_path="$1"

    [[ "${staged_path}" == *.rs ]] \
        || [[ "${staged_path}" == "Cargo.toml" ]] \
        || [[ "${staged_path}" == "Cargo.lock" ]]
}

real_script="$(resolve_script_path "$0")"
script_dir="$(cd "$(dirname "${real_script}")" && pwd)"
project_dir="$(cd "${script_dir}/.." && pwd)"

cd "${project_dir}"

should_run=false
while IFS= read -r -d '' staged_path; do
    if is_rust_commit_relevant_path "${staged_path}"; then
        should_run=true
        break
    fi
done < <(git -C "${project_dir}" diff --cached --name-only -z)

if [[ "${should_run}" != true ]]; then
    echo "==> No staged Rust or Cargo files detected, skipping."
    exit 0
fi

echo "==> Running LineEndings pre-commit checks..."

run cargo build --all-targets
run cargo clippy --all-targets --all-features -- -D clippy::all -D clippy::pedantic -F unsafe_code
run cargo fmt --check
run gtimeout "${NEXTEST_TIMEOUT_SECONDS}" cargo nextest run

echo "==> All checks passed."

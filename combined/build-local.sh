#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
repo="$(cd .. && pwd)"
if [[ -n "${MHGU_TOOLCHAIN:-}" ]]; then
    export RUSTUP_HOME="$MHGU_TOOLCHAIN/rustup"
    export CARGO_HOME="$MHGU_TOOLCHAIN/cargo"
fi
toolchain="${MHGU_RUST_TOOLCHAIN:-nightly-2026-11-02}"
mode="${1:---offline}"
case "$mode" in --offline|--online|--test-only) ;; *) echo 'Usage: build-local.sh [--offline|--online|--test-only]' >&2; exit 2;; esac
mkdir -p "$repo/build"
# Cargo splits RUSTFLAGS at whitespace. Encoded flags preserve paths with spaces.
encoded="${CARGO_ENCODED_RUSTFLAGS:-}"
if [[ -z "$encoded" && -n "${RUSTFLAGS:-}" ]]; then
    read -r -a inherited_flags <<< "$RUSTFLAGS"
    printf -v encoded '%s\x1f' "${inherited_flags[@]}"
    encoded="${encoded%$'\x1f'}"
fi
flags=(-C link-arg=-Tlink/switch32.ld -C link-arg=--build-id=sha1
    "--remap-path-prefix=$repo=/huntlog"
    "--remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo"
    "--remap-path-prefix=${RUSTUP_HOME:-$HOME/.rustup}=/rustup")
printf -v added_flags '%s\x1f' "${flags[@]}"
export CARGO_ENCODED_RUSTFLAGS="${encoded:+$encoded$'\x1f'}${added_flags%$'\x1f'}"
if [[ "$mode" != --test-only ]]; then
    cargo_args=(--release --locked)
    if [[ "$mode" == --offline ]]; then cargo_args+=(--offline); fi
    cargo +"$toolchain" build "${cargo_args[@]}"
    python3 ../tools/elf_to_nro.py target/armv7a-none-eabi/release/libmhgu_dps_meter.so ../build/hunter_hud.nro
fi
for unit in data layout target weakness recorder recorder_tests hunt_runtime_tests timers_tests player_details_tests hook_policy; do
    rustc +"$toolchain" --edition 2024 --test "src/$unit.rs" -o "../build/test_$unit"
    "../build/test_$unit" --test-threads=1
done

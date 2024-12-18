#!/usr/bin/env bash
set -euxo pipefail

rust3p_dir="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"
cargo_toml=${manifest_path:-$rust3p_dir/Cargo.toml}
vendor=${vendor:-$rust3p_dir/vendor/}
meson_build=${meson_build:-$rust3p_dir/meson.build}

cargo update --manifest-path "$cargo_toml"

cargo vendor \
    --manifest-path "$cargo_toml" \
    "$vendor"

cargo +nightly build \
    --manifest-path "$cargo_toml" \
    --config 'source.crates-io.replace-with="vendored-sources"' \
    --config "source.vendored-sources.directory=\"$vendor\"" \
    -Zunstable-options \
    --unit-graph \
    | cargo2meson --strip-prefix "$(dirname $vendor)" \
    > "$meson_build"

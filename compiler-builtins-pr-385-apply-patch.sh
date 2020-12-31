#!/bin/bash
patch="$(dirname "$(readlink -f "$0")")/compiler-builtins-pr-385.patch"
cd "$(dirname "$(dirname "$(rustup which rustc)")")/lib/rustlib/src/rust/vendor/compiler_builtins/"
patch -np1 < "$patch"

#!/bin/sh

set -eu

cargo run --quiet --bin actus-snapshot -- "$@"

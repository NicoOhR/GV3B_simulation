#!/bin/sh
cargo build --target x86_64-pc-windows-gnu &&
cp target/x86_64-pc-windows-gnu/debug/three_body.exe . &&
exec ./three_body.exe "$@"


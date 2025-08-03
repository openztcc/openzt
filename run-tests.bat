@echo off

REM Building openzt dll
cargo +nightly-2024-05-02-i686-pc-windows-msvc test --manifest-path openzt/Cargo.toml --release --target=i686-pc-windows-msvc --features "command-console,test-rpc" %*

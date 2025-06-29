@echo off

echo Opening docs
cargo +nightly rustdoc --manifest-path openzt/Cargo.toml --lib --target i686-pc-windows-msvc --open -- --document-private-items

if %errorlevel% neq 0 (
    echo Failed
    pause
    exit /b %errorlevel%
)

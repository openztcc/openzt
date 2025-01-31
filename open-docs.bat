@echo off

echo Opening docs
cargo +nightly rustdoc --lib --open -- --document-private-items

if %errorlevel% neq 0 (
    echo Failed
    pause
    exit /b %errorlevel%
)
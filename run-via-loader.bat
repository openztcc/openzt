@echo off
echo Building openzt dll
cargo +nightly build --lib --target=i686-pc-windows-msvc
if %errorlevel% neq 0 (
    echo Failed
    exit /b %errorlevel%
)

"../openzt-loader/target/release/openzt-loader.exe" --dll-path="target/i686-pc-windows-msvc/debug/openzt.dll" --listen --resume

pause
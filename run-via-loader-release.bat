@echo off
echo Building openzt dll
cargo +nightly build --lib --release --target=i686-pc-windows-msvc

if %errorlevel% neq 0 (
    echo Failed
    exit /b %errorlevel%
)

"../openzt-loader/target/release/openzt-loader.exe" --dll-path="target/i686-pc-windows-msvc/release/openzt.dll" --listen --resume

pause
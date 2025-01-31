@echo off

@REM TODO: build with specific nightly toolchain
echo Building openzt dll
cargo +nightly build --lib --target=i686-pc-windows-msvc

if %errorlevel% neq 0 (
    echo Failed
    pause
    exit /b %errorlevel%
)

"../openzt-loader/target/i686-pc-windows-msvc/release/openzt-loader.exe" --dll-path="target/i686-pc-windows-msvc/release/openzt.dll" --listen

pause
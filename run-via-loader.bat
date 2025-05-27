@echo off

@REM TODO: build with specific nightly toolchain
echo Building openzt dll
cargo +nightly build --lib --target=i686-pc-windows-msvc

if %errorlevel% neq 0 (
    echo Failed
    pause
    exit /b %errorlevel%
)

del "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\lang301-openzt.dll"

"../openzt-loader/target/i686-pc-windows-msvc/release/openzt-loader.exe" --dll-path="target/i686-pc-windows-msvc/debug/openzt.dll" --resume

pause
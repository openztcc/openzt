@echo off

echo Building openzt dll
cargo +nightly-2024-05-02-i686-pc-windows-msvc build --lib --release --target=i686-pc-windows-msvc %*

if %errorlevel% neq 0 (
    echo Failed
    pause
    exit /b %errorlevel%
)

del "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\lang301-openzt.dll"

"../openzt-loader/target/i686-pc-windows-msvc/release/openzt-loader.exe" --dll-path="target/i686-pc-windows-msvc/release/openzt.dll" --listen --resume

pause
@echo off

echo Building openzt dll
cargo +nightly build --lib --target=i686-pc-windows-msvc

if %errorlevel% neq 0 (
    echo Failed
    pause
    exit /b %errorlevel%
)

del "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\res-openzt.dll"
del "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\res-openztrpc.dll"

echo Building and running openzt loader
cargo +nightly run --manifest-path openzt-loader/Cargo.toml -- --dll-path="target/i686-pc-windows-msvc/debug/openzt.dll" --listen --resume

pause

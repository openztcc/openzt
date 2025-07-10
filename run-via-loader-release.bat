@echo off

echo Building openzt dll
cargo +nightly-2025-06-23-i686-pc-windows-msvc build --manifest-path openzt/Cargo.toml --lib --release --target=i686-pc-windows-msvc %*

if %errorlevel% neq 0 (
    echo Failed
    pause
    exit /b %errorlevel%
)

del "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\res-openzt.dll"
del "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\res-openztrpc.dll"

echo Building and running openzt loader
cargo +nightly-2025-06-23-i686-pc-windows-msvc run --release --manifest-path openzt-loader/Cargo.toml -- --dll-path="target/i686-pc-windows-msvc/release/openzt.dll" --resume

pause
@echo off

REM Building openzt dll
cargo +nightly-2024-05-02-i686-pc-windows-msvc build --manifest-path openzt-dll/Cargo.toml --lib --release --target=i686-pc-windows-msvc  --features "command-console" %*

if %errorlevel% neq 0 (
    echo Failed
    pause
    exit /b %errorlevel%
)

del "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\res-openzt.dll"
del "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\res-openztrpc.dll"

copy "target\i686-pc-windows-msvc\release\openzt.dll" "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\res-openzt.dll"

start "Zoo Tycoon" "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\zoo.exe"

pause
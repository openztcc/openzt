@echo off

echo Building openzt dll
cargo build --manifest-path openzt-test-dll/Cargo.toml --lib --release --target=i686-pc-windows-msvc %*

if %errorlevel% neq 0 (
    echo Failed
    pause
    exit /b %errorlevel%
)

del "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\res-openzt.dll"
del "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\res-openztrpc.dll"
del "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\res-openzttest.dll"

@REM echo Building and running openzt loader
@REM cargo +nightly-2025-06-23-i686-pc-windows-msvc run --release --manifest-path openzt-loader/Cargo.toml -- --dll-path="target\i686-pc-windows-msvc\release\openzttestrpc.dll" --resume

REM Copy the file to the destination

copy "target\i686-pc-windows-msvc\release\openzttest.dll" "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\res-openzttest.dll"

REM Check copy succeeded

if %ERRORLEVEL% neq 0 (
    echo Copy failed.
    pause
    exit /b
)

REM Run the zoo.exe executable
start "Zoo Tycoon" "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\zoo.exe"

pause
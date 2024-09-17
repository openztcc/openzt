@echo off

REM Run cargo build with specified arguments
cargo +nightly-2024-05-02-i686-pc-windows-msvc build --lib --release --target=i686-pc-windows-msvc %*

REM Check if the build succeeded
if %ERRORLEVEL% neq 0 (
    echo Cargo build failed.
    pause
    exit /b
)

REM Copy the file to the destination
copy "target\i686-pc-windows-msvc\release\openzt.dll" "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\lang301-openzt.dll"

REM Run the zoo.exe executable
start "Zoo Tycoon" "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\zoo.exe"

exit /b
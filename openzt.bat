@echo off
setlocal enabledelayedexpansion

REM OpenZT Unified Build Script
REM Combines build, run, and docs functionality

REM ============================================================
REM Main Dispatcher
REM ============================================================

IF "%~1"=="" GOTO show_help
IF "%~1"=="help" GOTO show_help
IF "%~1"=="--help" GOTO show_help
IF "%~1"=="-h" GOTO show_help
IF "%~1"=="build" GOTO parse_build_flags
IF "%~1"=="run" GOTO parse_run_flags
IF "%~1"=="docs" GOTO docs
IF "%~1"=="console" GOTO console
IF "%~1"=="check" GOTO check
IF "%~1"=="clippy" GOTO clippy
IF "%~1"=="test" GOTO test

echo Error: Unknown subcommand "%~1"
echo.
GOTO show_help

REM ============================================================
REM Parse Build Flags
REM ============================================================

:parse_build_flags
SET RUN_AFTER_BUILD=
SHIFT
GOTO parse_flags

:parse_run_flags
SET RUN_AFTER_BUILD=1
SHIFT
GOTO parse_flags

:parse_flags
SET RELEASE_FLAG=
SET TEST_FLAG=
SET STABLE_FLAG=
SET LOADER_FLAG=
SET PAUSE_FLAG=
SET CARGO_ARGS=
SET PARSING_CARGO_ARGS=

:parse_loop
IF "%~1"=="" GOTO validate_and_build
IF "%~1"=="--release" (
    SET RELEASE_FLAG=1
    SHIFT
    GOTO parse_loop
)
IF "%~1"=="--test" (
    SET TEST_FLAG=1
    SHIFT
    GOTO parse_loop
)
IF "%~1"=="--stable" (
    SET STABLE_FLAG=1
    SHIFT
    GOTO parse_loop
)
IF "%~1"=="--loader" (
    SET LOADER_FLAG=1
    SHIFT
    GOTO parse_loop
)
IF "%~1"=="--pause" (
    SET PAUSE_FLAG=1
    SHIFT
    GOTO parse_loop
)
IF "%~1"=="--" (
    SET PARSING_CARGO_ARGS=1
    SHIFT
    GOTO parse_loop
)
IF DEFINED PARSING_CARGO_ARGS (
    SET CARGO_ARGS=!CARGO_ARGS! %~1
    SHIFT
    GOTO parse_loop
)
echo Error: Unknown flag "%~1"
exit /b 1

:validate_and_build
REM Validate flag combinations
IF DEFINED PAUSE_FLAG (
    IF NOT DEFINED LOADER_FLAG (
        echo Error: --pause requires --loader
        pause
        exit /b 1
    )
)
GOTO build

REM ============================================================
REM Build Function
REM ============================================================

:build
REM Set toolchains
SET TOOLCHAIN=+nightly-2024-05-02-i686-pc-windows-msvc
SET LOADER_TOOLCHAIN=+nightly-2025-06-23-i686-pc-windows-msvc
IF DEFINED STABLE_FLAG (
    SET TOOLCHAIN=+stable
    SET LOADER_TOOLCHAIN=+stable
)

REM Set manifest path and DLL name
IF DEFINED TEST_FLAG (
    SET MANIFEST_PATH=openzt-test-dll/Cargo.toml
    SET DLL_NAME=openzttest.dll
    SET RUST_BACKTRACE=1
) ELSE (
    SET MANIFEST_PATH=openzt-dll/Cargo.toml
    SET DLL_NAME=openzt.dll
)

REM Set build type
SET BUILD_TYPE=debug
SET BUILD_FLAGS=
IF DEFINED RELEASE_FLAG (
    SET BUILD_TYPE=release
    SET BUILD_FLAGS=--release
)

REM Set feature flags - command-console by default for non-test nightly builds
SET FEATURE_FLAGS=
IF NOT DEFINED TEST_FLAG (
    IF NOT DEFINED STABLE_FLAG (
        SET FEATURE_FLAGS=--features "command-console"
    )
)

REM Display build info
echo Building !DLL_NAME! (!BUILD_TYPE!)...
IF DEFINED FEATURE_FLAGS (
    echo Features: !FEATURE_FLAGS!
)

REM Execute cargo build for DLL
cargo !TOOLCHAIN! build --manifest-path !MANIFEST_PATH! --lib --target=i686-pc-windows-msvc !BUILD_FLAGS! !FEATURE_FLAGS! !CARGO_ARGS!

IF !errorlevel! NEQ 0 (
    echo.
    echo Build failed
    pause
    exit /b !errorlevel!
)

echo.
echo Build successful: target\i686-pc-windows-msvc\!BUILD_TYPE!\!DLL_NAME!

REM Build loader if --loader flag is set
IF DEFINED LOADER_FLAG (
    echo.
    echo Building openzt-loader...
    cargo !LOADER_TOOLCHAIN! build --manifest-path openzt-loader/Cargo.toml --target=i686-pc-windows-msvc !BUILD_FLAGS!

    IF !errorlevel! NEQ 0 (
        echo.
        echo Loader build failed
        pause
        exit /b !errorlevel!
    )

    echo Loader build successful: target\i686-pc-windows-msvc\!BUILD_TYPE!\openzt-loader.exe
)

REM Write state file on success
echo BUILD_TYPE=!BUILD_TYPE! > .openzt-build-state
echo DLL_NAME=!DLL_NAME! >> .openzt-build-state

REM If run command was used, continue to copy and launch
IF DEFINED RUN_AFTER_BUILD GOTO copy_and_run
GOTO :EOF

REM ============================================================
REM Copy and Run Function
REM ============================================================

:copy_and_run
REM BUILD_TYPE and DLL_NAME are already set from the build step
REM Set source path
SET SOURCE_DLL=target\i686-pc-windows-msvc\!BUILD_TYPE!\!DLL_NAME!

REM Check source exists
IF NOT EXIST "!SOURCE_DLL!" (
    echo Error: Built DLL not found at !SOURCE_DLL!
    pause
    exit /b 1
)

REM Check if using loader injection
IF DEFINED LOADER_FLAG (
    echo.
    echo ============================================================
    echo WARNING: The --loader injection method is deprecated.
    echo Consider using the standard DLL copy method instead.
    echo ============================================================
    echo.

    REM Delete old DLLs
    echo Cleaning up old DLLs...
    del "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\res-openzt.dll" 2>nul
    del "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\res-openztrpc.dll" 2>nul
    del "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\res-openzttest.dll" 2>nul

    REM Run via loader
    IF DEFINED PAUSE_FLAG (
        echo.
        echo Running openzt-loader.exe directly ^(for debugger attachment^)...
        target\i686-pc-windows-msvc\!BUILD_TYPE!\openzt-loader.exe --dll-path=^"target/i686-pc-windows-msvc/!BUILD_TYPE!/!DLL_NAME!^"
    ) ELSE (
        echo.
        echo Running via openzt-loader ^(cargo run^)...
        cargo !LOADER_TOOLCHAIN! run !BUILD_FLAGS! --manifest-path openzt-loader/Cargo.toml -- --dll-path=^"target/i686-pc-windows-msvc/!BUILD_TYPE!/!DLL_NAME!^" --listen --resume
    )

    pause
    GOTO :EOF
)

REM Standard DLL copy method (no loader)
REM Delete old DLLs
echo.
echo Cleaning up old DLLs...
del "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\res-openzt.dll" 2>nul
del "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\res-openztrpc.dll" 2>nul
del "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\res-openzttest.dll" 2>nul

REM Determine destination name
IF !DLL_NAME!==openzt.dll (
    SET DEST_NAME=res-openzt.dll
) ELSE (
    SET DEST_NAME=res-openzttest.dll
)

REM Copy DLL
echo Copying !DLL_NAME! to Zoo Tycoon directory...
copy "!SOURCE_DLL!" "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\!DEST_NAME!"

IF !errorlevel! NEQ 0 (
    echo.
    echo Copy failed
    pause
    exit /b !errorlevel!
)

REM Launch game
echo.
echo Launching Zoo Tycoon...
start "Zoo Tycoon" "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\zoo.exe"

pause
GOTO :EOF

REM ============================================================
REM Docs Function
REM ============================================================

:docs
echo Opening documentation...
cargo +nightly rustdoc --manifest-path openzt/Cargo.toml --lib --target i686-pc-windows-msvc --open -- --document-private-items

IF !errorlevel! NEQ 0 (
    echo.
    echo Documentation generation failed
    pause
    exit /b !errorlevel!
)

GOTO :EOF

REM ============================================================
REM Console Function
REM ============================================================

:console
SHIFT
SET CONSOLE_ARGS=
:console_args_loop
IF "%~1"=="" GOTO run_console
SET CONSOLE_ARGS=!CONSOLE_ARGS! %1
SHIFT
GOTO console_args_loop

:run_console
IF "!CONSOLE_ARGS!"=="" (
    echo Opening console...
    cargo run --manifest-path=openzt-console/Cargo.toml
) ELSE (
    echo Running console command...
    cargo run --manifest-path=openzt-console/Cargo.toml --!CONSOLE_ARGS!
)

IF !errorlevel! NEQ 0 (
    echo.
    echo Console failed
    pause
    exit /b !errorlevel!
)

GOTO :EOF

REM ============================================================
REM Check Function
REM ============================================================

:check
echo Running cargo check on openzt...
cargo +nightly check --manifest-path openzt/Cargo.toml --target i686-pc-windows-msvc

IF !errorlevel! NEQ 0 (
    echo.
    echo Cargo check failed
    pause
    exit /b !errorlevel!
)

echo.
echo Cargo check passed
GOTO :EOF

REM ============================================================
REM Clippy Function
REM ============================================================

:clippy
echo Running cargo clippy on openzt...
cargo +nightly clippy --manifest-path openzt/Cargo.toml --target i686-pc-windows-msvc

IF !errorlevel! NEQ 0 (
    echo.
    echo Clippy found issues
    pause
    exit /b !errorlevel!
)

echo.
echo Clippy passed
GOTO :EOF

REM ============================================================
REM Test Function
REM ============================================================

:test
echo Running cargo test on openzt...
cargo +nightly test --manifest-path openzt/Cargo.toml --target i686-pc-windows-msvc

IF !errorlevel! NEQ 0 (
    echo.
    echo Tests failed
    pause
    exit /b !errorlevel!
)

echo.
echo Tests passed
GOTO :EOF

REM ============================================================
REM Help Function
REM ============================================================

:show_help
echo OpenZT Build Script
echo.
echo Usage: openzt.bat ^<subcommand^> [flags] [-- cargo-args]
echo.
echo Subcommands:
echo   build     Build the DLL only
echo   run       Build the DLL and launch the game
echo   check     Run cargo check on openzt crate
echo   clippy    Run cargo clippy on openzt crate
echo   test      Run cargo test on openzt crate
echo   docs      Generate and open documentation
echo   console   Open interactive Lua console or run oneshot command
echo   help      Show this help message
echo.
echo Build/Run Flags:
echo   --release      Build with release optimizations
echo   --test         Build the test DLL (openzt-test-dll)
echo   --stable       Build with stable Rust toolchain (disables command-console)
echo   --loader       Use openzt-loader for DLL injection (instead of copy)
echo   --pause        (with --loader) Run loader exe directly for debugger
echo   -- ^<args^>      Forward additional arguments to cargo
echo.
echo Note: command-console feature is enabled by default for non-test builds
echo       with nightly toolchain. Use --stable or --test to disable it.
echo.
echo Examples:
echo   openzt.bat build                     Build debug DLL with command-console
echo   openzt.bat build --release           Build release DLL with command-console
echo   openzt.bat build --loader            Build DLL + loader
echo   openzt.bat run                       Build debug, copy DLL, launch game
echo   openzt.bat run --release             Build release, copy DLL, launch game
echo   openzt.bat run --loader              Build + run via loader injection
echo   openzt.bat run --loader --pause      Build + run loader exe (for debugger)
echo   openzt.bat run --test                Build test DLL and launch game
echo   openzt.bat build --stable            Build debug without command-console
echo   openzt.bat check                     Run cargo check on openzt
echo   openzt.bat clippy                    Run cargo clippy on openzt
echo   openzt.bat test                      Run cargo test on openzt
echo   openzt.bat docs                      Generate and open docs
echo   openzt.bat console                   Open interactive Lua console
echo   openzt.bat console --oneshot "help()"          Run single Lua command and exit
echo   openzt.bat console --oneshot "add_cash(10000)" Add cash via oneshot command
echo.
GOTO :EOF

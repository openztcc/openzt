# Detour Testing Setup Guide

This guide provides quick instructions for running the automated detour testing system.

## Prerequisites

- Wine installed and configured
- Zoo Tycoon installed in Wine
- Rust nightly toolchain for i686-pc-windows-msvc target

## Quick Start

### 1. Build the Test DLL

```bash
./build_test_dll.sh
```

This builds OpenZT with detour testing features enabled and sets up the required environment variables.

### 2. Generate List of Detours to Test

```bash
./generate_detour_list.sh > detours_to_test.txt
```

This creates a prioritized list of detours that need testing (excludes already validated detours).

### 3. Configure Test Environment

Edit `test_detours.sh` to set your Zoo Tycoon path:

```bash
ZOO_TYCOON_PATH="/path/to/your/zoo.exe"
```

Or set it as an environment variable:

```bash
export ZOO_TYCOON_PATH="/home/user/.wine/drive_c/Program Files/Zoo Tycoon/zoo.exe"
```

### 4. Run Tests

```bash
./test_detours.sh
```

The script will:
- Test each detour individually
- Launch Zoo Tycoon for each test
- Wait for success signal or timeout
- Record results in `test_results/` directory

## Test Results

After testing completes, review the results:

- `test_results/successful_detours.txt` - Detours that were successfully called
- `test_results/failed_detours.txt` - Detours that failed or weren't called
- `test_results/test_log.txt` - Detailed test log with timestamps

## How It Works

1. **Build Phase**: The DLL is built with test features and environment variables pointing to config files
2. **Configuration**: Each test writes a single detour name to `detour_test_config.txt`
3. **Execution**: Zoo Tycoon launches and the DLL activates only the specified detour
4. **Signaling**: When the detour is called, it creates `detour_success.txt`
5. **Detection**: The script monitors for the success file and records the result

## Customization

### Adjust Timeout

```bash
TIMEOUT=60 ./test_detours.sh  # 60 second timeout per test
```

### Test Specific Detours

Create a custom list:

```bash
echo "BFAPP_CONSTRUCTOR" > detours_to_test.txt
echo "BFMAP_INIT" >> detours_to_test.txt
./test_detours.sh
```

### Change Result Directory

```bash
RESULTS_DIR=my_results ./test_detours.sh
```

## Troubleshooting

### Wine Issues

If Wine doesn't launch properly:
- Check WINE_PREFIX is set correctly
- Verify Zoo Tycoon runs manually: `wine /path/to/zoo.exe`
- Check Wine debug output: Remove `WINEDEBUG=-all` from script

### No Detours Detected

If all detours fail:
- Verify DLL is being loaded by checking Zoo Tycoon logs
- Check that openzt-loader is configured correctly
- Try testing a known-working detour like `BFAPP_WIN_MAIN`

### Build Errors

If the test DLL won't build:
- Ensure you have nightly Rust: `rustup default nightly`
- Install Windows target: `rustup target add i686-pc-windows-msvc`
- Check that once_cell dependency is available

## Next Steps

After successful testing:

1. **Investigate Failures**: Check failed detours for signature mismatches
2. **Add to Production**: Integrate successful detours into main codebase
3. **Create Unit Tests**: Add validated detours to automated test suite
4. **Document Issues**: Update detour definitions for any that need fixes
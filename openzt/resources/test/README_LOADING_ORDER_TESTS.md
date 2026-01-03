# Loading Order Integration Tests

This directory contains test resources for verifying deterministic loading order of OpenZT mod definition files.

## Test Mod: loading-order-test

The `loading-order-test` mod is used to test the deterministic loading order feature implemented in OpenZT. It contains multiple definition files across all three categories (NoPatch, Mixed, PatchOnly) to verify:

1. Category ordering (NoPatch → Mixed → PatchOnly)
2. Alphabetical sorting within each category
3. Case-insensitive sorting
4. Cross-file habitat references
5. Within-file habitat self-references
6. Patch execution order

### Mod Structure

```
loading-order-test/
├── meta.toml
└── defs/
    ├── 00-habitat-only.toml       (NoPatch)
    ├── 01-location-only.toml      (NoPatch)
    ├── 02-another-habitat.toml    (NoPatch)
    ├── Capitals-Test.toml         (NoPatch) - tests case-insensitive sorting
    ├── 50-mixed-content.toml      (Mixed)
    ├── 51-second-mixed.toml       (Mixed)
    ├── 98-early-patch.toml        (PatchOnly)
    └── 99-patches-only.toml       (PatchOnly)
```

### Expected Loading Order

1. **NoPatch files** (alphabetical):
   - `00-habitat-only.toml`
   - `01-location-only.toml`
   - `02-another-habitat.toml`
   - `Capitals-Test.toml`

2. **Mixed files** (alphabetical):
   - `50-mixed-content.toml`
   - `51-second-mixed.toml`

3. **PatchOnly files** (alphabetical):
   - `98-early-patch.toml`
   - `99-patches-only.toml`

## Running the Tests

### Build and Run

```bash
# Build with integration-tests feature
./openzt.bat build --release -- --features integration-tests

# Run tests (game will launch and exit automatically)
./openzt.bat run --release -- --features integration-tests
```

**No manual setup required!** The test mod is embedded in the binary and loaded automatically.

### Test Output

The game will load, run all tests, and exit automatically. Check the log file at:
```
C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\openzt_integration_tests.log
```

### Expected Output

```
=== OpenZT Integration Tests ===

Loading embedded test mod: loading-order-test
Test mod loaded successfully

Running patch rollback tests...
  ✓ test_continue_mode_applies_directly
  ... (9 tests)

Running loading order tests...
  ✓ test_category_ordering
  ✓ test_alphabetical_within_nopatch
  ✓ test_alphabetical_within_mixed
  ✓ test_alphabetical_within_patchonly
  ✓ test_case_insensitive_sorting
  ✓ test_cross_file_habitat_reference
  ✓ test_mixed_file_self_reference
  ✓ test_patch_execution_order

Results: 17 passed, 0 failed
ALL TESTS PASSED
```

## Modifying Test Resources

Test resources are embedded directly in the binary using `include_str!()` and `include_bytes!()` macros. To modify tests:

1. Edit the TOML files in `openzt/resources/test/loading-order-test/defs/`
2. Rebuild the DLL with `./openzt.bat build --features integration-tests`
3. Run tests to verify changes

No ZIP file creation or installation is required - changes are automatically embedded during compilation.

## Test Coverage

| Test | Category | Purpose |
|------|----------|---------|
| `test_category_ordering` | Integration | Verifies NoPatch → Mixed → PatchOnly order |
| `test_alphabetical_within_nopatch` | Integration | Verifies alphabetical sorting in NoPatch category |
| `test_alphabetical_within_mixed` | Integration | Verifies alphabetical sorting in Mixed category |
| `test_alphabetical_within_patchonly` | Integration | Verifies alphabetical sorting in PatchOnly category |
| `test_case_insensitive_sorting` | Integration | Verifies case-insensitive sorting |
| `test_cross_file_habitat_reference` | Functional | Verifies PatchOnly file can reference NoPatch habitat |
| `test_mixed_file_self_reference` | Functional | Verifies Mixed file can reference its own habitat |
| `test_patch_execution_order` | Functional | Verifies later patches override earlier patches |

## Notes

- The tests require a running game environment with initialized memory structures
- Test resources are embedded in the binary at compile time (zero runtime overhead)
- Tests create temporary files (`animals/test.ai`, `animals/test_order.ai`) for verification
- The load order tracker only records events when the `integration-tests` feature is enabled
- Icon resources are reused from `openzt/resources/test/` directory
- The ZIP file (`loading-order-test.zip`) is no longer required and can be safely deleted

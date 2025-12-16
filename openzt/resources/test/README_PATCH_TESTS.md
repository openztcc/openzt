# Patch System Test Scenarios

## Test Structure

Each test scenario should verify:
1. Setup: Initial resource state
2. Action: Patch application
3. Verification: Expected resource state
4. Cleanup: Reset if needed

## Scenario Groups

### Group 1: Basic Operations
- Test each of 13 patch operations individually
- Verify file modifications persist in resource system
- Check logs for success messages

### Group 2: Error Handling (Continue Mode Only)
- Scenario 2.1: Continue mode - patch fails, next patch executes
- Scenario 2.2: Validate abort/abort_mod modes return error

### Group 3: Conditionals
- Scenario 3.1: Top-level condition.target with key_exists
- Scenario 3.2: Top-level condition without target (should error)
- Scenario 3.3: Patch-level condition overrides default target
- Scenario 3.4: Multiple conditions (AND logic)

### Group 4: Array Semantics
- Scenario 4.1: append_value to single-value key creates array
  * Setup: [Section] Key=value1
  * Patch: append_value "value2"
  * Verify: Two "Key=" entries in INI
- Scenario 4.2: append_values to existing array
  * Setup: [Section] Key=a, Key=b
  * Patch: append_values ["c", "d"]
  * Verify: Four "Key=" entries

### Group 5: Integration
- Scenario 5.1: Mod loading triggers patch application
- Scenario 5.2: Cross-mod patches (Mod B patches Mod A's files)
- Scenario 5.3: Load order dependency

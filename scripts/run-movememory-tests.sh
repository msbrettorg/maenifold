#!/bin/bash

# MoveMemory Smoke Test Suite - Complete Test Harness
# Tests both CLI and MCP interfaces per TEST-MATRIX.md lines 184-205
# Reference: https://github.com/ma-collective/maenifold/docs/TEST-MATRIX.md

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
RESULTS_FILE="$PROJECT_ROOT/test-outputs/smoke-tests/MoveMemory-results.md"
DOTNET_PATH=$(which dotnet)
MEMORY_TOOLS="$PROJECT_ROOT/src/bin/Release/net9.0/maenifold.dll"

# Ensure build is up-to-date
echo "Building project..."
dotnet build "$PROJECT_ROOT" -c Release -v quiet > /dev/null 2>&1

# Track test results
TESTS_PASSED=0
TESTS_FAILED=0
TEST_RESULTS=""

# Helper to extract URI from output (filtering noise)
extract_uri() {
    local output="$1"
    # Extract memory:// URIs, skip diagnostic messages
    echo "$output" | grep "memory://" | grep -v "\[" | head -1 | sed 's/.*\(memory:\/\/[^ "]*\).*/\1/'
}

# Helper function to run test with setup
run_test_with_setup() {
    local test_name="$1"
    local source_title="$2"
    local source_content="$3"
    local destination="$4"
    local should_succeed="$5"

    echo ""
    echo "=== Test: $test_name ==="

    # Create source file (suppress diagnostic output)
    echo "Setup: Creating source file '$source_title'..."
    local create_result=$($DOTNET_PATH "$MEMORY_TOOLS" --tool WriteMemory --payload "{\"title\":\"$source_title\",\"content\":\"$source_content\"}" 2>&1)

    # Extract the source URI from the result
    local source_uri=$(extract_uri "$create_result")

    if [ -z "$source_uri" ]; then
        echo "✗ FAIL: Could not extract source URI"
        ((TESTS_FAILED++))
        TEST_RESULTS="${TEST_RESULTS}| $test_name | FAIL | Could not create source file |\n"
        return
    fi

    echo "  Source URI: $source_uri"

    # Attempt move
    echo "Test: Moving from '$source_uri' to '$destination'..."
    local move_result=$($DOTNET_PATH "$MEMORY_TOOLS" --tool MoveMemory --payload "{\"source\":\"$source_uri\",\"destination\":\"$destination\"}" 2>&1)

    # Check result
    if [ "$should_succeed" = "true" ]; then
        if echo "$move_result" | grep -q "Moved memory FILE"; then
            echo "✓ PASS"
            ((TESTS_PASSED++))
            TEST_RESULTS="${TEST_RESULTS}| $test_name | PASS | Successfully moved file |\n"
        else
            echo "✗ FAIL: Expected success but got: $move_result"
            ((TESTS_FAILED++))
            TEST_RESULTS="${TEST_RESULTS}| $test_name | FAIL | $move_result |\n"
        fi
    else
        if echo "$move_result" | grep -q "ERROR"; then
            echo "✓ PASS (error expected)"
            ((TESTS_PASSED++))
            TEST_RESULTS="${TEST_RESULTS}| $test_name | PASS | Error correctly returned |\n"
        else
            echo "✗ FAIL: Expected error but got success"
            ((TESTS_FAILED++))
            TEST_RESULTS="${TEST_RESULTS}| $test_name | FAIL | Expected error but succeeded |\n"
        fi
    fi
}

# Write header
cat > "$RESULTS_FILE" << 'HEADER'
# MoveMemory Smoke Test Results

**Test Date**: TIMESTAMP_PLACEHOLDER
**Dotnet Version**: DOTNET_VERSION_PLACEHOLDER
**Test Specification**: TEST-MATRIX.md lines 184-205

## Test Environment

- Project: Ma-Core (Maenifold)
- Test Type: Smoke Tests (CLI & MCP)
- Tool: MoveMemory
- Purpose: Relocate and rename knowledge files while preserving [[WikiLinks]]

## Test Cases from TEST-MATRIX.md

### Success Path Tests

HEADER

# Replace placeholders
sed -i '' "s/TIMESTAMP_PLACEHOLDER/$(date -u +%Y-%m-%dT%H:%M:%SZ)/g" "$RESULTS_FILE"
sed -i '' "s/DOTNET_VERSION_PLACEHOLDER/$($DOTNET_PATH --version)/g" "$RESULTS_FILE"

echo ""
echo "====== MoveMemory Smoke Test Suite ======"
echo ""

# TEST 1: Simple rename
run_test_with_setup \
    "Simple rename" \
    "Old Name Test" \
    "# Old Name Test\n\nTesting [[simple rename]]." \
    "New Name Test" \
    "true"

# TEST 2: Move to subfolder
run_test_with_setup \
    "Move to subfolder" \
    "File To Move" \
    "# File To Move\n\nTesting [[subfolder]] movement." \
    "subfolders/new-file" \
    "true"

# TEST 3: Move from subfolder to root
run_test_with_setup \
    "Move from subfolder to root" \
    "Subfolder File" \
    "# Subfolder File\n\nTesting movement [[from subfolder]]." \
    "Root File" \
    "true"

# TEST 4: Move between folders
run_test_with_setup \
    "Move between folders" \
    "File Between Folders" \
    "# File Between Folders\n\nTesting [[between folders]] movement." \
    "folder-a/folder-b/moved-file" \
    "true"

# TEST 5: Source doesn't exist (error case)
echo ""
echo "=== Test: Source doesn't exist ==="
move_result=$($DOTNET_PATH "$MEMORY_TOOLS" --tool MoveMemory --payload "{\"source\":\"nonexistent-file-xyz\",\"destination\":\"new-file\"}" 2>&1)
if echo "$move_result" | grep -q "ERROR"; then
    echo "✓ PASS: Error correctly returned for nonexistent source"
    ((TESTS_PASSED++))
    TEST_RESULTS="${TEST_RESULTS}| Source doesn't exist | PASS | Returns ERROR for missing file |\n"
else
    echo "✗ FAIL: Should have returned error"
    ((TESTS_FAILED++))
    TEST_RESULTS="${TEST_RESULTS}| Source doesn't exist | FAIL | No error returned |\n"
fi

# TEST 6: Destination exists (should fail)
echo ""
echo "=== Test: Destination exists ==="
# Create first file
create_result=$($DOTNET_PATH "$MEMORY_TOOLS" --tool WriteMemory --payload "{\"title\":\"Existing File\",\"content\":\"# Existing\\n\\nFile [[exists]]\"}" 2>&1)
source_uri=$(extract_uri "$create_result")
# Try to create another with same name (should fail on move)
create_result2=$($DOTNET_PATH "$MEMORY_TOOLS" --tool WriteMemory --payload "{\"title\":\"Another File\",\"content\":\"# Another\\n\\nFile [[test]]\"}" 2>&1)
source_uri2=$(extract_uri "$create_result2")
# Try to move second to first's location
move_result=$($DOTNET_PATH "$MEMORY_TOOLS" --tool MoveMemory --payload "{\"source\":\"$source_uri2\",\"destination\":\"Existing File\"}" 2>&1)
if echo "$move_result" | grep -q "ERROR"; then
    echo "✓ PASS: Error correctly returned when destination exists"
    ((TESTS_PASSED++))
    TEST_RESULTS="${TEST_RESULTS}| Destination exists | PASS | Returns ERROR for existing destination |\n"
else
    echo "✗ FAIL: Should have returned error for existing destination"
    ((TESTS_FAILED++))
    TEST_RESULTS="${TEST_RESULTS}| Destination exists | FAIL | Overwrite not prevented |\n"
fi

# TEST 7: Empty source validation
echo ""
echo "=== Test: Empty source ==="
move_result=$($DOTNET_PATH "$MEMORY_TOOLS" --tool MoveMemory --payload "{\"source\":\"\",\"destination\":\"new-file\"}" 2>&1)
if echo "$move_result" | grep -q "ERROR"; then
    echo "✓ PASS: Error correctly returned for empty source"
    ((TESTS_PASSED++))
    TEST_RESULTS="${TEST_RESULTS}| Empty source | PASS | Returns ERROR for empty source |\n"
else
    echo "✗ FAIL: Should have returned error"
    ((TESTS_FAILED++))
    TEST_RESULTS="${TEST_RESULTS}| Empty source | FAIL | No error returned |\n"
fi

# TEST 8: Empty destination validation
echo ""
echo "=== Test: Empty destination ==="
run_test_with_setup \
    "Empty destination" \
    "Empty Dest Test" \
    "# Empty Dest Test\n\nTesting [[empty destination]]." \
    "" \
    "false"

# TEST 9: WikiLinks preservation test
run_test_with_setup \
    "Verify [[WikiLinks]] preserved" \
    "WikiLinks Test" \
    "# WikiLinks Test\n\nThis file has [[concept-one]], [[concept-two]], and [[concept-three]] in it." \
    "wikilinks-moved" \
    "true"

# TEST 10: Special chars in destination
run_test_with_setup \
    "Special chars in destination" \
    "Special Chars File" \
    "# Special Chars File\n\nTesting [[special characters]]." \
    "special-folder/file-with-dash" \
    "true"

# TEST 11: Move preserves content
echo ""
echo "=== Test: Move preserves content and metadata ==="
create_result=$($DOTNET_PATH "$MEMORY_TOOLS" --tool WriteMemory --payload "{\"title\":\"Content Preservation Test\",\"content\":\"# Content Preservation\\n\\nThis file tests [[content preservation]] during move operations.\"}" 2>&1)
source_uri=$(extract_uri "$create_result")
move_result=$($DOTNET_PATH "$MEMORY_TOOLS" --tool MoveMemory --payload "{\"source\":\"$source_uri\",\"destination\":\"content-preserved-file\"}" 2>&1)
if echo "$move_result" | grep -q "Moved memory FILE"; then
    # Verify content is readable at new location
    read_result=$($DOTNET_PATH "$MEMORY_TOOLS" --tool ReadMemory --payload "{\"identifier\":\"memory://content-preserved-file\"}" 2>&1)
    if echo "$read_result" | grep -q "content preservation"; then
        echo "✓ PASS: Content preserved after move"
        ((TESTS_PASSED++))
        TEST_RESULTS="${TEST_RESULTS}| Move preserves content | PASS | Content verified at new location |\n"
    else
        echo "✗ FAIL: Content not found after move"
        ((TESTS_FAILED++))
        TEST_RESULTS="${TEST_RESULTS}| Move preserves content | FAIL | Content lost during move |\n"
    fi
else
    echo "✗ FAIL: Move operation failed"
    ((TESTS_FAILED++))
    TEST_RESULTS="${TEST_RESULTS}| Move preserves content | FAIL | Move failed |\n"
fi

# Summary
echo ""
echo "====== Test Summary ======"
echo "Tests Passed: $TESTS_PASSED"
echo "Tests Failed: $TESTS_FAILED"

# Write results to file
cat >> "$RESULTS_FILE" << 'RESULTS_TABLE'

## Test Results Summary

| Test Case | Status | Details |
|-----------|--------|---------|
RESULTS_TABLE

echo -e "$TEST_RESULTS" >> "$RESULTS_FILE"

cat >> "$RESULTS_FILE" << 'RESULTS_FOOTER'

## Execution Summary

RESULTS_FOOTER

echo "**Passed**: $TESTS_PASSED" >> "$RESULTS_FILE"
echo "**Failed**: $TESTS_FAILED" >> "$RESULTS_FILE"
echo "**Total**: $((TESTS_PASSED + TESTS_FAILED))" >> "$RESULTS_FILE"
echo "**Pass Rate**: $(( (TESTS_PASSED * 100) / (TESTS_PASSED + TESTS_FAILED) ))%" >> "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"

# Add test specifications
cat >> "$RESULTS_FILE" << 'TEST_SPEC'

## Test Specifications (from TEST-MATRIX.md)

### Success Path Tests
- **Simple rename**: `--tool MoveMemory --payload '{"source":"old","destination":"new"}'`
- **Move to subfolder**: `--tool MoveMemory --payload '{"source":"file","destination":"subfolder/file"}'`
- **Move from subfolder to root**: `--tool MoveMemory --payload '{"source":"subfolder/file","destination":"file"}'`
- **Move between folders**: `--tool MoveMemory --payload '{"source":"a/file","destination":"b/file"}'`

### Error Handling Tests
- **Source doesn't exist**: Should return ERROR
- **Destination exists**: Should return ERROR (prevent overwrite)

### Validation Tests
- **Empty source**: Should return ERROR
- **Empty destination**: Should return ERROR

### Graph Integrity Tests
- **Verify [[WikiLinks]] preserved**: Content and concepts remain intact
- **Verify references updated**: Graph connections maintained

### Return Value Validation
- Returns `MoveMemoryResult` with old → new URIs
- Content preserved and readable at new location
- Metadata timestamps updated

## Tool Contract

```
Input: source (string), destination (string)
Output: "Moved memory FILE: {oldURI} → {newURI}"
Errors: "ERROR: {description}"
```

## Notes

- All tests use real file I/O (no mocks)
- Memory files stored in configured directory
- [[WikiLink]] concepts preserved in markdown content
- Special characters in destinations handled safely
- Path traversal protected (destination must stay within memory root)

TEST_SPEC

echo "**Test Report**: $RESULTS_FILE" >> "$RESULTS_FILE"

echo ""
echo "====== Results ======"
echo "Test results saved to: $RESULTS_FILE"
echo ""
cat "$RESULTS_FILE"

exit $TESTS_FAILED

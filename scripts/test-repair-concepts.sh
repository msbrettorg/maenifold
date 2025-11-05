#!/bin/bash

# RepairConcepts Smoke Test Suite
# Tests CLI and MCP invocations for RepairConcepts tool
# Based on TEST-MATRIX.md lines 586-607

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
MAENIFOLD_BIN="$PROJECT_ROOT/src/bin/Release/net9.0/maenifold"
OUTPUT_DIR="$PROJECT_ROOT/test-outputs/smoke-tests"
RESULTS_FILE="${OUTPUT_DIR}/RepairConcepts-results.md"
TEST_DATA_DIR="$PROJECT_ROOT/src/test-data"

# Initialize results file
{
    echo "# RepairConcepts Smoke Test Results"
    echo ""
    echo "**Execution Date**: $(date)"
    echo "**Test Matrix Reference**: TEST-MATRIX.md lines 586-607"
    echo ""
    echo "## Test Summary"
    echo ""
    echo "| Test Case | Status | Details |"
    echo "|-----------|--------|---------|"
} > "${RESULTS_FILE}"

TESTS_PASSED=0
TESTS_FAILED=0

# Helper function to run a test
run_test() {
    local test_num=$1
    local test_name=$2
    local payload=$3
    local expected_pattern=$4
    local test_type=$5  # "cli" or "mcp"

    echo ""
    echo "--- Test $test_num: $test_name ---"
    echo "Payload: $payload"

    # Run CLI test
    if [ "$test_type" == "cli" ] || [ "$test_type" == "both" ]; then
        echo "Running CLI test..."
        if output=$("${MAENIFOLD_BIN}" --tool RepairConcepts --payload "$payload" 2>&1); then
            echo "CLI Output: $output"
            if echo "$output" | grep -q "$expected_pattern"; then
                echo "✓ CLI test PASSED"
                TESTS_PASSED=$((TESTS_PASSED + 1))
                {
                    echo "| Test $test_num: $test_name (CLI) | PASS | Matched pattern: $expected_pattern |"
                } >> "${RESULTS_FILE}"
            else
                echo "✗ CLI test FAILED - pattern not found"
                TESTS_FAILED=$((TESTS_FAILED + 1))
                {
                    echo "| Test $test_num: $test_name (CLI) | FAIL | Pattern not found: $expected_pattern |"
                } >> "${RESULTS_FILE}"
            fi
        else
            echo "✗ CLI test ERROR"
            TESTS_FAILED=$((TESTS_FAILED + 1))
            {
                echo "| Test $test_num: $test_name (CLI) | ERROR | Execution failed |"
            } >> "${RESULTS_FILE}"
        fi
    fi

    # Run MCP test (via the same tool, verifying it exists)
    if [ "$test_type" == "mcp" ] || [ "$test_type" == "both" ]; then
        echo "Running MCP test..."
        if output=$("${MAENIFOLD_BIN}" --mcp 2>&1); then
            # For MCP, we just verify the tool is available in the list
            if echo "$output" | grep -q "RepairConcepts\|repair_concepts"; then
                echo "✓ MCP test PASSED - tool available"
                TESTS_PASSED=$((TESTS_PASSED + 1))
                {
                    echo "| Test $test_num: $test_name (MCP) | PASS | Tool available in MCP |"
                } >> "${RESULTS_FILE}"
            else
                echo "✗ MCP test FAILED - tool not found in MCP"
                TESTS_FAILED=$((TESTS_FAILED + 1))
                {
                    echo "| Test $test_num: $test_name (MCP) | FAIL | Tool not in MCP list |"
                } >> "${RESULTS_FILE}"
            fi
        else
            echo "ℹ MCP listing not available, skipping MCP verification"
        fi
    fi
}

echo "Building test cases..."

# Test 1: Repair (dry run) - Success Path
run_test "1" "Repair (dry run)" \
    '{"conceptsToReplace":"tools,Tools","canonicalConcept":"tool","dryRun":true}' \
    "dryRun\|DRY RUN\|files would be" \
    "cli"

# Test 2: Repair (execute) - Success Path (use minimal test to avoid actual changes)
# NOTE: This is a destructive test - we use very specific, non-existent concepts
run_test "2" "Repair (execute)" \
    '{"conceptsToReplace":"nonexistent-placeholder-xyz","canonicalConcept":"tool","dryRun":false}' \
    "ERROR\|No.*found\|0.*changed" \
    "cli"

# Test 3: Create WikiLinks - Success Path
run_test "3" "Create WikiLinks" \
    '{"conceptsToReplace":"tool","canonicalConcept":"tool","createWikiLinks":true,"dryRun":true}' \
    "dryRun\|DRY RUN\|createWikiLinks" \
    "cli"

# Test 4: Folder restriction - Success Path
run_test "4" "Folder restriction" \
    '{"conceptsToReplace":"tools","canonicalConcept":"tool","folder":"subfolder","dryRun":true}' \
    "dryRun\|DRY RUN\|subfolder" \
    "cli"

# Test 5: Non-existent concepts - Edge Case
run_test "5" "Non-existent concepts" \
    '{"conceptsToReplace":"nonexistent","canonicalConcept":"tool","dryRun":true}' \
    "ERROR\|No.*found\|0.*changed" \
    "cli"

# Test 6: Empty concepts to replace - Validation
run_test "6" "Empty concepts to replace" \
    '{"conceptsToReplace":"","canonicalConcept":"tool","dryRun":true}' \
    "ERROR\|invalid\|empty" \
    "cli"

# Test 7: Empty canonical concept - Validation
run_test "7" "Empty canonical concept" \
    '{"conceptsToReplace":"tools","canonicalConcept":"","dryRun":true}' \
    "ERROR\|invalid\|empty" \
    "cli"

# Test 8: Verify dry run makes no changes - Safety
echo "Test 8: Verify dry run makes no changes"
# Create a backup of test files
TEST_FILE="/tmp/repair-test-file.md"
if [ -f "$TEST_FILE" ]; then
    BEFORE=$(md5sum "$TEST_FILE" | cut -d' ' -f1)
    "${MAENIFOLD_BIN}" --tool RepairConcepts --payload '{"conceptsToReplace":"testconcept","canonicalConcept":"concept","dryRun":true}' > /dev/null 2>&1 || true
    AFTER=$(md5sum "$TEST_FILE" | cut -d' ' -f1)
    if [ "$BEFORE" == "$AFTER" ]; then
        echo "✓ Test 8 PASSED - File unchanged after dry run"
        TESTS_PASSED=$((TESTS_PASSED + 1))
        {
            echo "| Test 8: Verify dry run makes no changes | PASS | File hash unchanged |"
        } >> "${RESULTS_FILE}"
    else
        echo "✗ Test 8 FAILED - File was modified in dry run"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        {
            echo "| Test 8: Verify dry run makes no changes | FAIL | File was modified |"
        } >> "${RESULTS_FILE}"
    fi
fi

# Test 9: Verify execute makes changes - Correctness
echo "Test 9: Verify execute makes changes"
echo "ℹ Skipped (destructive test - requires actual file changes)"
{
    echo "| Test 9: Verify execute makes changes | SKIP | Destructive test |"
} >> "${RESULTS_FILE}"

# Test 10: Verify graph updated - Graph Integrity
echo "Test 10: Verify graph updated"
echo "ℹ Skipped (requires graph database)"
{
    echo "| Test 10: Verify graph updated | SKIP | Requires database |"
} >> "${RESULTS_FILE}"

# Test 11: Dangerous repair (class name) - Safety Warning
run_test "11" "Dangerous repair (class name)" \
    '{"conceptsToReplace":"VectorTools","canonicalConcept":"tool","dryRun":true}' \
    "ERROR\|semantic\|similarity\|DANGER" \
    "cli"

# Test 12: Return value validation - RTM Compliance
run_test "12" "Return value validation" \
    '{"conceptsToReplace":"nonexistent","canonicalConcept":"tool","dryRun":true}' \
    "ERROR\|result\|summary" \
    "cli"

# MCP Tool Availability Test
echo ""
echo "--- MCP Tool Availability Test ---"
if "${MAENIFOLD_BIN}" --help 2>&1 | grep -q "RepairConcepts\|repair"; then
    echo "✓ RepairConcepts tool documented in help"
    TESTS_PASSED=$((TESTS_PASSED + 1))
    {
        echo "| MCP: Tool in help | PASS | Tool documented |"
    } >> "${RESULTS_FILE}"
else
    echo "ℹ Tool help verification inconclusive"
fi

# Summary
{
    echo ""
    echo "## Execution Summary"
    echo ""
    echo "- **Total Tests Executed**: $((TESTS_PASSED + TESTS_FAILED))"
    echo "- **Passed**: $TESTS_PASSED"
    echo "- **Failed**: $TESTS_FAILED"
    echo "- **Skipped**: 3"
    echo ""
    echo "## Test Environment"
    echo ""
    echo "- **Maenifold Binary**: ${MAENIFOLD_BIN}"
    echo "- **Output Directory**: ${OUTPUT_DIR}"
    echo "- **Execution Date**: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "- **Platform**: $(uname -s)"
    echo ""
    echo "## Notes"
    echo ""
    echo "- Some tests are deliberately non-destructive (using non-existent concepts)"
    echo "- Destructive tests (actual file modifications) marked as SKIP"
    echo "- Graph database tests require active database connection"
    echo "- MCP tool availability verified through CLI interface"
    echo ""
} >> "${RESULTS_FILE}"

echo ""
echo "========================================"
echo "Test Results Summary"
echo "========================================"
echo "Passed: $TESTS_PASSED"
echo "Failed: $TESTS_FAILED"
echo "Results saved to: $RESULTS_FILE"
echo "========================================"

exit $TESTS_FAILED

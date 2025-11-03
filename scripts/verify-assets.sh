#!/bin/bash

# Verification script for asset JSON files
# Checks for required fields: id, name, description
# Reports issues with shortDescription or missing fields

total_files=0
passed_files=0
failed_files=0
issues=()

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=== Asset JSON Verification ==="
echo ""

# Function to check a JSON file
check_json_file() {
    local file="$1"
    local category="$2"

    ((total_files++))

    # Check if file is valid JSON
    if ! jq empty "$file" 2>/dev/null; then
        issues+=("${RED}MALFORMED JSON${NC}: $file")
        ((failed_files++))
        return 1
    fi

    # Extract fields
    local id=$(jq -r '.id // empty' "$file")
    local name=$(jq -r '.name // empty' "$file")
    local description=$(jq -r '.description // empty' "$file")
    local shortDescription=$(jq -r '.shortDescription // empty' "$file")
    local emoji=$(jq -r '.emoji // empty' "$file")

    local file_issues=()

    # Check required fields
    if [ -z "$id" ]; then
        file_issues+=("missing 'id'")
    fi

    if [ -z "$name" ]; then
        file_issues+=("missing 'name'")
    fi

    if [ -z "$description" ]; then
        file_issues+=("missing 'description'")
    fi

    # Check for deprecated shortDescription
    if [ -n "$shortDescription" ]; then
        file_issues+=("has deprecated 'shortDescription' field")
    fi

    # Note if emoji is missing (warning for perspectives, note for others)
    if [ -z "$emoji" ] && [ "$category" != "perspectives" ]; then
        file_issues+=("missing 'emoji' (optional)")
    fi

    # Report results
    if [ ${#file_issues[@]} -eq 0 ]; then
        echo -e "${GREEN}✓${NC} $file"
        ((passed_files++))
    else
        echo -e "${RED}✗${NC} $file"
        for issue in "${file_issues[@]}"; do
            echo -e "  ${YELLOW}→${NC} $issue"
            issues+=("$file: $issue")
        done
        ((failed_files++))
    fi
}

# Check workflows
echo "--- Workflows ---"
for file in ~/maenifold/assets/workflows/*.json; do
    check_json_file "$file" "workflows"
done
echo ""

# Check roles
echo "--- Roles ---"
for file in ~/maenifold/assets/roles/*.json; do
    check_json_file "$file" "roles"
done
echo ""

# Check colors
echo "--- Colors ---"
for file in ~/maenifold/assets/colors/*.json; do
    check_json_file "$file" "colors"
done
echo ""

# Check perspectives
echo "--- Perspectives ---"
for file in ~/maenifold/assets/perspectives/*.json; do
    check_json_file "$file" "perspectives"
done
echo ""

# Summary
echo "=== Summary ==="
echo "Total files checked: $total_files"
echo -e "${GREEN}Passed: $passed_files${NC}"
echo -e "${RED}Failed: $failed_files${NC}"
echo ""

if [ $failed_files -gt 0 ]; then
    echo "=== Issues Found ==="
    printf '%s\n' "${issues[@]}"
    exit 1
else
    echo -e "${GREEN}✓ All asset files have required fields and proper structure!${NC}"
    exit 0
fi

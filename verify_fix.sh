#!/bin/bash

# Exit on error
set -e

echo "--- Verification Script Started ---"

# 1. Clean up
echo "--- Cleaning up old data..."
rm -rf docs storage
mkdir -p docs

# 2. Setup test file
echo "--- Creating test file..."
echo "This is a line for the test." > docs/test.txt

# Give a moment for the file system to be ready
sleep 1

# 3. Initial Run
echo "--- Building and performing initial run and search..."
cargo build > /dev/null 2>&1
CARGO_RUN_OUTPUT_1=$( (echo "line"; echo "quit") | ./target/debug/ai_search_demo )
echo "$CARGO_RUN_OUTPUT_1"

# Check for single occurrence
# The title of the doc is the filename without extension, so "test"
NUM_RESULTS_1=$(echo "$CARGO_RUN_OUTPUT_1" | grep -c "📄 \[test\]")
if [ "$NUM_RESULTS_1" -ne 1 ]; then
    echo "ERROR: Expected 1 search result on initial run, but found $NUM_RESULTS_1."
    exit 1
fi
echo "--- Initial run successful."

# 4. Modify File
echo "--- Modifying test file..."
echo "This is the second line." >> docs/test.txt

# 5. Second Run (after modification)
echo "--- Performing second run and search..."
# A small sleep to ensure the file modification timestamp is different
sleep 1
CARGO_RUN_OUTPUT_2=$( (echo "line"; echo "quit") | ./target/debug/ai_search_demo )
echo "$CARGO_RUN_OUTPUT_2"

# 6. Assert
echo "--- Verifying results..."
NUM_RESULTS_2=$(echo "$CARGO_RUN_OUTPUT_2" | grep -c "📄 \[test\]")

if [ "$NUM_RESULTS_2" -eq 1 ]; then
    echo "SUCCESS: Found 1 search result after modification, as expected."
    rm ./verify_fix.sh
    exit 0
else
    echo "FAILURE: Expected 1 search result after modification, but found $NUM_RESULTS_2."
    rm ./verify_fix.sh
    exit 1
fi

#!/bin/bash
# Setup real-world test repositories for validation

set -e

cd test_repos

echo "Setting up real-world test repositories..."

# Small/Medium projects with good test suites
repos=(
    "https://github.com/psf/requests.git"
    "https://github.com/pallets/click.git"
    "https://github.com/pallets/markupsafe.git"
    "https://github.com/pallets/itsdangerous.git"
    "https://github.com/benoitc/httplib2.git"
)

for repo in "${repos[@]}"; do
    project_name=$(basename "$repo" .git)
    if [ ! -d "$project_name" ]; then
        echo "Cloning $project_name..."
        git clone --depth 1 "$repo"
    else
        echo "$project_name already exists, skipping..."
    fi
done

echo "Test repositories setup complete!"

# Create a validation script
cat > validate_all.sh << 'EOF'
#!/bin/bash
# Validate Fastest against all test repositories

set -e

FASTEST_BIN="../target/release/fastest"
RESULTS_FILE="validation_results.md"

echo "# Real-World Test Validation Results" > "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"
echo "Date: $(date)" >> "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"

for project in */; do
    if [ -d "$project" ]; then
        project_name=$(basename "$project")
        echo "Testing $project_name..."
        
        echo "## $project_name" >> "$RESULTS_FILE"
        echo "" >> "$RESULTS_FILE"
        
        # Find test directory
        if [ -d "$project/tests" ]; then
            test_dir="$project/tests"
        elif [ -d "$project/test" ]; then
            test_dir="$project/test"
        else
            echo "No test directory found for $project_name" >> "$RESULTS_FILE"
            continue
        fi
        
        # Count tests
        test_count=$(find "$test_dir" -name "test_*.py" -o -name "*_test.py" | wc -l)
        echo "Test files: $test_count" >> "$RESULTS_FILE"
        
        # Run with Fastest
        echo "### Fastest Results" >> "$RESULTS_FILE"
        if timeout 60s "$FASTEST_BIN" "$test_dir" > "${project_name}_fastest.log" 2>&1; then
            tail -n 20 "${project_name}_fastest.log" >> "$RESULTS_FILE"
        else
            echo "FAILED or TIMEOUT" >> "$RESULTS_FILE"
            echo "See ${project_name}_fastest.log for details" >> "$RESULTS_FILE"
        fi
        
        echo "" >> "$RESULTS_FILE"
        echo "---" >> "$RESULTS_FILE"
        echo "" >> "$RESULTS_FILE"
    fi
done

echo "Validation complete! Results in $RESULTS_FILE"
EOF

chmod +x validate_all.sh

echo "Created validation script: test_repos/validate_all.sh"
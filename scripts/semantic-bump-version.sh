#!/bin/bash
# Script to bump version across all Rust workspace members and Python files
# Called by semantic-release during the prepare phase

set -e

NEW_VERSION=$1

if [ -z "$NEW_VERSION" ]; then
    echo "Error: Version argument required"
    echo "Usage: $0 <new_version>"
    exit 1
fi

echo "Bumping version to $NEW_VERSION"

# Function to update version in TOML files
update_toml_version() {
    local file=$1
    
    if [ -f "$file" ]; then
        # Handle workspace version
        if grep -q '^\[workspace\]' "$file" && grep -q '^version = ' "$file"; then
            # Update workspace version
            if [[ "$OSTYPE" == "darwin"* ]]; then
                sed -i '' "/^\[workspace\]/,/^\[.*\]/{s/^version = \".*\"/version = \"$NEW_VERSION\"/}" "$file"
            else
                sed -i "/^\[workspace\]/,/^\[.*\]/{s/^version = \".*\"/version = \"$NEW_VERSION\"/}" "$file"
            fi
        fi
        
        # Handle package version
        if grep -q '^\[package\]' "$file" && grep -q '^version = ' "$file"; then
            # Update package version (unless it inherits from workspace)
            if ! grep -q 'version\.workspace = true' "$file"; then
                if [[ "$OSTYPE" == "darwin"* ]]; then
                    sed -i '' "/^\[package\]/,/^\[.*\]/{s/^version = \".*\"/version = \"$NEW_VERSION\"/}" "$file"
                else
                    sed -i "/^\[package\]/,/^\[.*\]/{s/^version = \".*\"/version = \"$NEW_VERSION\"/}" "$file"
                fi
            fi
        fi
        
        echo "  ✓ Updated $file"
    fi
}

# Update root Cargo.toml (workspace)
update_toml_version "Cargo.toml"

# Update all crate Cargo.toml files
for crate_toml in crates/*/Cargo.toml; do
    if [ -f "$crate_toml" ]; then
        update_toml_version "$crate_toml"
    fi
done

# Update pyproject.toml
if [ -f "pyproject.toml" ]; then
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "s/^version = \".*\"/version = \"$NEW_VERSION\"/" pyproject.toml
    else
        sed -i "s/^version = \".*\"/version = \"$NEW_VERSION\"/" pyproject.toml
    fi
    echo "  ✓ Updated pyproject.toml"
fi

# Update setup.py
if [ -f "setup.py" ]; then
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "s/version=\".*\"/version=\"$NEW_VERSION\"/" setup.py
    else
        sed -i "s/version=\".*\"/version=\"$NEW_VERSION\"/" setup.py
    fi
    echo "  ✓ Updated setup.py"
fi

# Update Cargo.lock by running cargo check
echo "Updating Cargo.lock..."
cargo check --quiet || true

echo "Version bumped to $NEW_VERSION successfully"
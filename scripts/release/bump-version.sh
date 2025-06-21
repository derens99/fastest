#!/bin/bash
# Script to bump version across all project files

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get current version from Cargo.toml
CURRENT_VERSION=$(grep -m1 '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')

if [ -z "$1" ]; then
    echo -e "${RED}Error: Version argument required${NC}"
    echo "Usage: $0 <new_version>"
    echo "Current version: $CURRENT_VERSION"
    echo ""
    echo "Examples:"
    echo "  $0 0.2.1    # For a patch release"
    echo "  $0 0.3.0    # For a minor release"
    echo "  $0 1.0.0    # For a major release"
    exit 1
fi

NEW_VERSION=$1

# Validate version format
if ! [[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo -e "${RED}Error: Invalid version format${NC}"
    echo "Version must be in format X.Y.Z (e.g., 0.2.1)"
    exit 1
fi

echo -e "${YELLOW}Updating version from $CURRENT_VERSION to $NEW_VERSION${NC}"
echo ""

# Function to update version in a file
update_version() {
    local file=$1
    local pattern=$2
    local replacement=$3
    
    if [ -f "$file" ]; then
        if grep -q "$pattern" "$file"; then
            if [[ "$OSTYPE" == "darwin"* ]]; then
                # macOS
                sed -i '' "s/$pattern/$replacement/g" "$file"
            else
                # Linux
                sed -i "s/$pattern/$replacement/g" "$file"
            fi
            echo -e "  ${GREEN}✓${NC} Updated $file"
        else
            echo -e "  ${YELLOW}⚠${NC} Pattern not found in $file"
        fi
    else
        echo -e "  ${RED}✗${NC} File not found: $file"
    fi
}

# Update Cargo.toml (workspace root)
update_version "Cargo.toml" "^version = \"$CURRENT_VERSION\"" "version = \"$NEW_VERSION\""

# Update pyproject.toml
update_version "pyproject.toml" "version = \"$CURRENT_VERSION\"" "version = \"$NEW_VERSION\""

# Update setup.py
update_version "setup.py" "version=\"$CURRENT_VERSION\"" "version=\"$NEW_VERSION\""

# Update CLI version test if it exists
update_version "crates/fastest-cli/tests/integration_test.rs" "$CURRENT_VERSION" "$NEW_VERSION"

# Update documentation files
update_version "README.md" "version $CURRENT_VERSION" "version $NEW_VERSION"
update_version "docs/README.md" "version $CURRENT_VERSION" "version $NEW_VERSION"
update_version "CHANGELOG.md" "$CURRENT_VERSION" "$NEW_VERSION"

echo ""
echo -e "${GREEN}Version updated to $NEW_VERSION${NC}"
echo ""
echo "Next steps:"
echo "1. Review the changes: git diff"
echo "2. Update CHANGELOG.md with release notes"
echo "3. Commit: git commit -am \"chore: bump version to $NEW_VERSION\""
echo "4. Tag: git tag -a v$NEW_VERSION -m \"Release v$NEW_VERSION\""
echo "5. Push: git push && git push --tags"
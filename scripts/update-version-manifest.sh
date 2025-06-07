#!/usr/bin/env bash
# Update version manifest after a release

set -e

# Arguments
VERSION="$1"
TAG="$2"

if [[ -z "$VERSION" || -z "$TAG" ]]; then
    echo "Usage: $0 <version> <tag>"
    echo "Example: $0 0.3.0 v0.3.0"
    exit 1
fi

# Remove 'v' prefix if present
VERSION="${VERSION#v}"

# Version manifest file
MANIFEST_FILE=".github/version.json"

# Get current date
DATE=$(date +%Y-%m-%d)

# Platforms to include
PLATFORMS=(
    "x86_64-unknown-linux-gnu"
    "aarch64-unknown-linux-gnu" 
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
    "x86_64-pc-windows-msvc"
)

# Create downloads and checksums objects
DOWNLOADS="{"
CHECKSUMS="{"
FIRST=true

for target in "${PLATFORMS[@]}"; do
    ext="tar.gz"
    if [[ "$target" == *"windows"* ]]; then
        ext="zip"
    fi
    
    if [[ "$FIRST" == true ]]; then
        FIRST=false
    else
        DOWNLOADS+=","
        CHECKSUMS+=","
    fi
    
    # upload-rust-binary-action uses format: fastest-$tag-$target
    DOWNLOADS+="\"$target\": \"https://github.com/derens99/fastest/releases/download/$TAG/fastest-$TAG-$target.$ext\""
    CHECKSUMS+="\"$target\": \"https://github.com/derens99/fastest/releases/download/$TAG/fastest-$TAG-$target.$ext.sha256sum\""
done

DOWNLOADS+="}"
CHECKSUMS+="}"

# Check if version.json exists
if [[ -f "$MANIFEST_FILE" ]]; then
    # Read existing content
    EXISTING_CONTENT=$(cat "$MANIFEST_FILE")
    
    # Update latest version
    TEMP_FILE=$(mktemp)
    
    # Use jq to update the JSON
    if command -v jq &> /dev/null; then
        # Update using jq
        echo "$EXISTING_CONTENT" | jq \
            --arg version "$VERSION" \
            --arg date "$DATE" \
            --argjson downloads "$DOWNLOADS" \
            --argjson checksums "$CHECKSUMS" \
            '.latest = $version | .versions[$version] = {
                date: $date,
                downloads: $downloads,
                checksums: $checksums
            }' > "$TEMP_FILE"
        
        # Pretty print and save
        jq . "$TEMP_FILE" > "$MANIFEST_FILE"
        rm "$TEMP_FILE"
    else
        # Manual update without jq
        echo "Warning: jq not found, creating new version.json"
        cat > "$MANIFEST_FILE" << EOF
{
  "latest": "$VERSION",
  "minimum": "0.1.0",
  "versions": {
    "$VERSION": {
      "date": "$DATE",
      "downloads": $DOWNLOADS,
      "checksums": $CHECKSUMS
    }
  }
}
EOF
    fi
else
    # Create new version.json
    cat > "$MANIFEST_FILE" << EOF
{
  "latest": "$VERSION",
  "minimum": "0.1.0",
  "versions": {
    "$VERSION": {
      "date": "$DATE",
      "downloads": $DOWNLOADS,
      "checksums": $CHECKSUMS
    }
  }
}
EOF
fi

echo "✅ Updated $MANIFEST_FILE with version $VERSION"
echo "📋 Contents:"
cat "$MANIFEST_FILE"
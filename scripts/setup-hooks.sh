#!/bin/bash
# Setup Git hooks for the project

echo "Setting up Git hooks..."
git config core.hooksPath .githooks
echo "✅ Git hooks configured!"
echo ""
echo "The pre-push hook will now run automatically before pushing to GitHub."
echo "To bypass the hook in emergencies, use: git push --no-verify"
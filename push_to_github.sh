#!/bin/bash

# Script to push all commits to GitHub
set -e  # Exit on any error

echo "🚀 Pushing all commits to GitHub..."

# Check if we're on the master branch
current_branch=$(git branch --show-current)
echo "📍 Current branch: $current_branch"

# Push to GitHub
echo "📤 Pushing to origin/$current_branch..."
git push origin $current_branch

echo "✅ Successfully pushed all commits to GitHub!"
echo ""
echo "🔗 You can view the commits on GitHub at:"
echo "   https://github.com/$(git config --get remote.origin.url | sed 's/.*github.com[:/]\([^.]*\).*/\1/')/commits/$current_branch"

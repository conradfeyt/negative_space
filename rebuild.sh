#!/bin/bash
# rebuild.sh — Kill app, bump build number, full bundle build, open from dist.
set -e
export PATH="/opt/homebrew/bin:/usr/local/bin:$HOME/.cargo/bin:$PATH"
cd "$(dirname "$0")"

# Kill running instances
pkill -x "Negativ_" 2>/dev/null || true
pkill -x "negative-space" 2>/dev/null || true
sleep 1

# Bump build number
BUILD_FILE="src/buildNumber.ts"
CURRENT=$(grep -o 'BUILD_NUMBER = [0-9]*' "$BUILD_FILE" | grep -o '[0-9]*')
NEXT=$((CURRENT + 1))
sed -i '' "s/BUILD_NUMBER = $CURRENT/BUILD_NUMBER = $NEXT/" "$BUILD_FILE"
echo "Build $NEXT"

# Full bundle build
npm run tauri build 2>&1 | tail -5

# Open from dist
open src-tauri/target/release/bundle/macos/Negativ_.app

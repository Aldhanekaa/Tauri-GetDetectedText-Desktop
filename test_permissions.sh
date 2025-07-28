#!/bin/bash

# Test accessibility permissions
echo "Testing accessibility permissions..."

# This will trigger the permission dialog if not already granted
osascript -e 'tell application "System Events" to get name of every process' > /dev/null 2>&1

if [ $? -eq 0 ]; then
    echo "✅ Accessibility permissions are granted!"
else
    echo "❌ Accessibility permissions are NOT granted."
    echo "Please grant permissions in System Settings > Privacy & Security > Accessibility"
    echo "Add your terminal app and the Tauri development app."
fi

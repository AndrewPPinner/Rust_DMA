#!/bin/bash

# Remote destination
REMOTE="192.168.1.106:~/code/Rust_DMA"

# Name of folder to temporarily move
TARGET_FOLDER="target"

# Check if target exists
if [ -d "$TARGET_FOLDER" ]; then
    echo "Moving $TARGET_FOLDER out of the way..."
    mv "$TARGET_FOLDER" ../
    TARGET_MOVED=true
else
    TARGET_MOVED=false
fi

# Copy current directory to remote
echo "Starting SCP..."
scp -r . "$REMOTE"

# Move target back if it was moved
if [ "$TARGET_MOVED" = true ]; then
    echo "Restoring $TARGET_FOLDER..."
    mv ../"$TARGET_FOLDER" ./
fi

echo "Done."

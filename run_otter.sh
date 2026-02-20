#!/bin/bash
# Otter - Easy Launcher for Linux/macOS
# This script makes it simple to run Otter without manual setup

echo "================================================"
echo "     Otter - Decentralized Private Chat"
echo "================================================"
echo

# Load environment variables from .env file if it exists
if [ -f ".env" ]; then
    echo "Loading environment variables from .env..."
    export $(cat .env | grep -v '^#' | xargs)
else
    echo "Note: .env file not found. If you need to use Google OAuth, please:"
    echo "  1. Copy .env.example to .env"
    echo "  2. Add your Google OAuth credentials"
    echo "  See SETUP_OAUTH.md for instructions."
    echo
fi

# Check if otter binary exists
if [ -f "./otter" ]; then
    echo "✓ Found otter binary"
    echo
    
    # Make sure it's executable
    chmod +x ./otter
    
    echo "Starting Otter..."
    echo
    
    # Run otter
    ./otter
    
else
    echo "✗ ERROR: otter binary not found in current directory"
    echo
    echo "Please ensure you have:"
    echo "  1. Downloaded the complete Otter release package"
    echo "  2. Extracted all files to the same directory"
    echo "  3. You are running this script from that directory"
    echo
    echo "If you need to build from source:"
    echo "  cargo build --release -p otter-cli"
    echo "  cp target/release/otter ."
    echo
    exit 1
fi

echo
echo "Otter has exited."

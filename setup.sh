#!/usr/bin/env bash
set -euo pipefail

echo "=== commoncrawletl setup ==="

# Detect OS
OS="$(uname -s)"

# Install Rust if not present
if command -v rustc &>/dev/null; then
    echo "Rust already installed: $(rustc --version)"
else
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo "Rust installed: $(rustc --version)"
fi

# Install system dependencies
if [ "$OS" = "Linux" ]; then
    if command -v apt-get &>/dev/null; then
        echo "Installing system dependencies (apt)..."
        sudo apt-get update -qq
        sudo apt-get install -y -qq build-essential pkg-config aria2
    elif command -v dnf &>/dev/null; then
        echo "Installing system dependencies (dnf)..."
        sudo dnf install -y gcc make pkg-config aria2
    elif command -v yum &>/dev/null; then
        echo "Installing system dependencies (yum)..."
        sudo yum install -y gcc make pkgconfig aria2
    else
        echo "Warning: Could not detect package manager. Ensure build-essential and aria2 are installed."
    fi
elif [ "$OS" = "Darwin" ]; then
    if command -v brew &>/dev/null; then
        echo "Installing system dependencies (brew)..."
        brew install aria2 2>/dev/null || true
    else
        echo "Warning: Homebrew not found. Install aria2 manually for faster downloads."
    fi
fi

# Build the project
echo "Building commoncrawletl (release mode)..."
cargo build --release

echo ""
echo "=== Setup complete ==="
echo "Binary at: ./target/release/commoncrawletl"
echo "Next step: run ./download.sh to fetch the WDC Event data"

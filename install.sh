#!/usr/bin/env bash
set -e

echo "🚀 Installing Apex Package Manager..."

# Check if cargo/rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust (cargo) is required to build Apex from source."
    echo "👉 Install it first using: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Clone repository if not already inside it
if [ ! -f "Cargo.toml" ]; then
    echo "📦 Cloning Apex repository..."
    git clone https://github.com/YOUR_GITHUB_USERNAME/apex.git /tmp/apex-build
    cd /tmp/apex-build
fi

echo "⚙️ Building optimized release binary..."
cargo build --release

echo "📥 Installing binary to /usr/local/bin..."
sudo cp target/release/apex /usr/local/bin/apex

echo "✨ Apex installed successfully! Try running: apex --version"

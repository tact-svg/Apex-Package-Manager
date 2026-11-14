#!/usr/bin/env bash
set -e

# Colors for nice output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 Installing Apex Package Manager...${NC}"

# 1. Check if cargo/rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust (cargo) is required to build Apex from source."
    echo "👉 Install it first using: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# 2. Build or compile binary
if [ -f "Cargo.toml" ]; then
    echo -e "${BLUE}⚙️ Building optimized release binary locally...${NC}"
    cargo build --release
    BINARY_PATH="target/release/apex"
else
    echo "❌ Cargo.toml not found. Please run this script from the Apex repository root."
    exit 1
fi

# 3. Setup installation directory (No sudo required!)
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

echo -e "${BLUE}📥 Installing apex binary to $INSTALL_DIR...${NC}"
cp "$BINARY_PATH" "$INSTALL_DIR/apex"
chmod +x "$INSTALL_DIR/apex"

# 4. Automatically handle PATH setup for Bash and Zsh
TARGET_PATH_LINE='export PATH="$HOME/.local/bin:$PATH"'

added_to_shell=false

for rc in "$HOME/.bashrc" "$HOME/.zshrc"; do
    if [ -f "$rc" ]; then
        if ! grep -q "$HOME/.local/bin" "$rc"; then
            echo "" >> "$rc"
            echo "# Added by Apex Package Manager" >> "$rc"
            echo "$TARGET_PATH_LINE" >> "$rc"
            echo -e "${GREEN}✔ Added Apex to PATH in $rc${NC}"
            added_to_shell=true
        fi
    fi
done

echo -e "${GREEN}✨ Apex installed successfully!${NC}"
echo ""
echo "To get started right now, run:"
echo -e "  ${BLUE}export PATH=\"\$HOME/.local/bin:\$PATH\"${NC}"
echo "  ${BLUE}apex --help${NC}"

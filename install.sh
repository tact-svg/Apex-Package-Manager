#!/usr/bin/env bash
set -e

# Colors for nice output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 Installing Apex Package Manager...${NC}"

# 1. Load Cargo environment or check common paths explicitly
if [ -f "$HOME/.cargo/env" ]; then
    . "$HOME/.cargo/env"
fi

# Fallback explicit path check
if ! command -v cargo &> /dev/null && [ -f "$HOME/.cargo/bin/cargo" ]; then
    export PATH="$HOME/.cargo/bin:$PATH"
fi

if ! command -v cargo &> /dev/null; then
    echo "❌ Rust (cargo) is required to build Apex from source."
    echo "👉 Install it first using: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# 2. Clone repo into a temporary directory if not running from the repository root
if [ ! -f "Cargo.toml" ]; then
    echo -e "${BLUE}📦 Downloading Apex source code...${NC}"
    TMP_DIR=$(mktemp -d)
    git clone https://github.com/tact-svg/Apex-Package-Manager.git "$TMP_DIR"
    cd "$TMP_DIR"
fi

# 3. Build optimized release binary
echo -e "${BLUE}⚙️ Building optimized release binary locally...${NC}"
cargo build --release
BINARY_PATH="target/release/apex"

if [ ! -f "$BINARY_PATH" ]; then
    echo "❌ Build failed. Binary not found."
    exit 1
fi

# 4. Setup installation directory (No sudo required!)
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

echo -e "${BLUE}📥 Installing apex binary to $INSTALL_DIR...${NC}"
cp "$BINARY_PATH" "$INSTALL_DIR/apex"
chmod +x "$INSTALL_DIR/apex"

# 5. Automatically handle PATH setup for Bash and Zsh
TARGET_PATH_LINE='export PATH="$HOME/.local/bin:$PATH"'

for rc in "$HOME/.bashrc" "$HOME/.zshrc"; do
    if [ -f "$rc" ]; then
        if ! grep -q "$HOME/.local/bin" "$rc"; then
            echo "" >> "$rc"
            echo "# Added by Apex Package Manager" >> "$rc"
            echo "$TARGET_PATH_LINE" >> "$rc"
            echo -e "${GREEN}✔ Added Apex to PATH in $rc${NC}"
        fi
    fi
done

echo -e "${GREEN}✨ Apex installed successfully!${NC}"
echo ""
echo "To get started right now, run:"
echo -e "  ${BLUE}export PATH=\"\$HOME/.local/bin:\$PATH\"${NC}"
echo "  ${BLUE}apex --help${NC}"

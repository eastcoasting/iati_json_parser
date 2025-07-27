#!/bin/bash

# IATI JSON Parser Installation Script

echo "🚀 Installing IATI JSON Parser..."

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust is not installed. Please install Rust first:"
    echo "   Visit https://rustup.rs/ or run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Check if Python is installed
if ! command -v python3 &> /dev/null; then
    echo "❌ Python 3 is not installed. Please install Python 3.8 or higher."
    exit 1
fi

# Install maturin if not already installed
if ! python3 -c "import maturin" &> /dev/null; then
    echo "📦 Installing maturin..."
    pip3 install maturin
fi

# Build and install the package
echo "🔨 Building and installing the package..."
maturin develop --release

echo "✅ Installation complete!"
echo ""
echo "Usage:"
echo "  python3 -c \"import iati_json_parser; print('Ready to use!')\""
echo ""
echo "Or in your Python code:"
echo "  from iati_json_parser import convert"
echo "  result = convert('path/to/xml/file.xml')" 
#!/bin/bash
set -e

echo "Building Ollama Interface AppImage..."

# 1. Build the Rust application
echo "Building Rust application..."
cargo build --release

# 2. Create AppDir structure
echo "Creating AppDir structure..."
mkdir -p AppDir/usr/{bin,share/{applications,icons/hicolor/256x256/apps}}

# 3. Copy the binary to AppDir
echo "Copying binary to AppDir..."
cp target/release/ollama-interface AppDir/usr/bin/

# 4. Save the icon to the right location
echo "Saving icon..."
cat > AppDir/usr/share/icons/hicolor/256x256/apps/ollama-interface.svg << 'EOF'
<svg viewBox="0 0 256 256" xmlns="http://www.w3.org/2000/svg">
  <rect width="256" height="256" rx="28" fill="#4299e1"/>
  <circle cx="128" cy="128" r="80" fill="#ffffff" fill-opacity="0.2"/>
  <circle cx="100" cy="108" r="20" fill="#ffffff"/>
  <circle cx="156" cy="108" r="20" fill="#ffffff"/>
  <path d="M88 160 C88 160, 128 190, 168 160" stroke="#ffffff" stroke-width="12" stroke-linecap="round" fill="none"/>
</svg>
EOF

# Convert SVG to PNG (optional, if needed)
if command -v convert &> /dev/null; then
  echo "Converting SVG to PNG..."
  convert -background none AppDir/usr/share/icons/hicolor/256x256/apps/ollama-interface.svg AppDir/usr/share/icons/hicolor/256x256/apps/ollama-interface.png
fi

# 5. Create a desktop entry
echo "Creating desktop entry..."
cat > AppDir/usr/share/applications/ollama-interface.desktop << EOF
[Desktop Entry]
Name=Ollama Interface
Exec=ollama-interface
Icon=ollama-interface
Type=Application
Categories=Utility;
Comment=A Rust-based interface for Ollama models
EOF

# IMPORTANT: Copy desktop file to AppDir root (required by appimagetool)
echo "Copying desktop file to AppDir root..."
cp AppDir/usr/share/applications/ollama-interface.desktop AppDir/

# 6. Create a symlink for AppRun
echo "Creating AppRun symlink..."
ln -sf usr/bin/ollama-interface AppDir/AppRun

# IMPORTANT: Copy icon to AppDir root (required by appimagetool)
echo "Copying icon to AppDir root..."
cp AppDir/usr/share/icons/hicolor/256x256/apps/ollama-interface.png AppDir/ollama-interface.png

# 7. Download and use appimagetool manually without SSL verification
echo "Downloading appimagetool..."
wget --no-check-certificate -c https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage -O appimagetool-x86_64.AppImage || {
  echo "Failed to download appimagetool with wget. Trying with curl..."
  curl -k -L https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage -o appimagetool-x86_64.AppImage
}

# Make it executable
echo "Making appimagetool executable..."
chmod +x appimagetool-x86_64.AppImage

# 8. Build the AppImage
echo "Building the AppImage..."
./appimagetool-x86_64.AppImage AppDir Ollama_Interface-x86_64.AppImage

echo "AppImage created successfully: Ollama_Interface-x86_64.AppImage"
echo "You can run it with: ./Ollama_Interface-x86_64.AppImage"
#!/bin/bash

BINARY_NAME="brc3"
INSTALL_DIR="/usr/local/bin"
ICON_NAME="bedrock-icon.png"

echo "Locating $BINARY_NAME..."
FOUND_PATH=$(find . -name "$BINARY_NAME" -type f 2>/dev/null | head -n 1)
[ -z "$FOUND_PATH" ] && FOUND_PATH=$(find /home /usr/local -name "$BINARY_NAME" -type f 2>/dev/null | head -n 1)

if [ -z "$FOUND_PATH" ]; then
    echo "Error: $BINARY_NAME executable was not found."
    echo "Please download the release version from the GitHub repository."
    exit 1
fi

echo "Installing $BINARY_NAME to $INSTALL_DIR..."
sudo cp "$FOUND_PATH" "$INSTALL_DIR/$BINARY_NAME"
sudo chmod +x "$INSTALL_DIR/$BINARY_NAME"

echo "Locating $ICON_NAME..."
FOUND_ICON=$(find . -name "$ICON_NAME" -type f 2>/dev/null | head -n 1)
[ -z "$FOUND_ICON" ] && FOUND_ICON=$(find /home -name "$ICON_NAME" -type f 2>/dev/null | head -n 1)

if [ -z "$FOUND_ICON" ]; then
    echo "Error: $ICON_NAME was not found."
    exit 1
fi

echo "Cleaning conflicting local user MIME caches..."
rm -rf ~/.local/share/mime
rm -f ~/.local/share/applications/bedrock.desktop

echo "Installing icon into system GTK pixmaps fallback..."
sudo mkdir -p /usr/share/pixmaps
sudo cp "$FOUND_ICON" /usr/share/pixmaps/application-x-br.png
sudo cp "$FOUND_ICON" /usr/share/pixmaps/bedrock-icon.png

echo "Installing icon across system theme resolutions..."
SIZES=("16x16" "22x22" "24x24" "32x32" "48x48" "64x64" "128x128" "256x256" "scalable")
for SIZE in "${SIZES[@]}"; do
    sudo mkdir -p /usr/share/icons/hicolor/${SIZE}/mimetypes
    sudo cp "$FOUND_ICON" /usr/share/icons/hicolor/${SIZE}/mimetypes/application-x-br.png
    
    if [ -d "/usr/share/icons/Mint-Y" ]; then
        sudo mkdir -p /usr/share/icons/Mint-Y/mimetypes/${SIZE} 2>/dev/null || true
        sudo cp "$FOUND_ICON" /usr/share/icons/Mint-Y/mimetypes/${SIZE}/application-x-br.png 2>/dev/null || true
    fi
done

echo "Registering system-wide MIME definition..."
sudo mkdir -p /usr/share/mime/packages
sudo bash -c 'cat << EOF > /usr/share/mime/packages/bedrock.xml
<?xml version="1.0" encoding="UTF-8"?>
<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
  <mime-type type="application/x-br">
    <comment>BedRock Source File</comment>
    <glob pattern="*.br"/>
    <icon name="application-x-br"/>
    <generic-icon name="application-x-br"/>
  </mime-type>
</mime-info>
EOF'

echo "Registering application entry..."
sudo bash -c 'cat << EOF > /usr/share/applications/bedrock.desktop
[Desktop Entry]
Type=Application
Name=BedRock Language
Exec=bedrockco %f
Icon=application-x-br
MimeType=application/x-br;
Terminal=true
Categories=Development;
EOF'

echo "Updating system databases and icon caches..."
sudo update-mime-database /usr/share/mime
sudo update-desktop-database /usr/share/applications
sudo gtk-update-icon-cache -f -q /usr/share/icons/hicolor
[ -d "/usr/share/icons/Mint-Y" ] && sudo gtk-update-icon-cache -f -q /usr/share/icons/Mint-Y

xdg-mime default bedrock.desktop application/x-br 2>/dev/null || true

echo "Clearing thumbnail cache and restarting Nemo..."
rm -rf ~/.cache/thumbnails/* 2>/dev/null || true
nemo -q 2>/dev/null || true

echo "Installation complete."

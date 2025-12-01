# Icon Placeholder
This directory should contain the application icons.

Generate icons from the SVG file using:
```bash
# Install imagemagick
sudo pacman -S imagemagick

# Generate PNGs from SVG
convert -background none -resize 32x32 icon.svg 32x32.png
convert -background none -resize 128x128 icon.svg 128x128.png
convert -background none -resize 256x256 icon.svg 128x128@2x.png
```

For Linux, these icons are optional. The application will work without them.

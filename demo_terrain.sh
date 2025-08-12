#!/bin/bash

echo "===================================="
echo "Mapper Terrain Generator Demo"
echo "===================================="
echo ""
echo "Generating a terrain map with seed 99999..."
echo ""

# Generate map with specific seed
echo -e "2\n99999\n4" | ./target/debug/mapper-terrain-cli | head -60

echo ""
echo "===================================="
echo "PNG Image Information:"
echo "===================================="

if [ -f "terrain_map_99999.png" ]; then
    file terrain_map_99999.png
    ls -lh terrain_map_99999.png
    echo ""
    echo "The high-resolution PNG has been saved!"
    echo "You can open it with any image viewer."
    echo ""
    echo "Example: xdg-open terrain_map_99999.png  # Linux"
    echo "         open terrain_map_99999.png      # macOS"
    echo "         start terrain_map_99999.png     # Windows"
else
    echo "PNG generation failed or file not found."
fi
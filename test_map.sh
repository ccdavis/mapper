#!/bin/bash
echo "Generating test map..."
echo -e "1" | timeout 3 ./target/release/mapper-terrain-cli 2>/dev/null | sed -n '/Terrain Features/,/Select option/p' | head -30
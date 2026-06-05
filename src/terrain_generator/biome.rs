use serde::{Deserialize, Serialize};

use super::TerrainGenerator;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Biome {
    DeepOcean,
    Ocean,
    Shore,
    Beach,
    Plains,
    Forest,
    Hills,
    Mountains,
    SnowPeaks,
    River,
    Lake,
    Swamp,
    Desert,
}

impl Biome {
    pub fn is_water(&self) -> bool {
        matches!(
            self,
            Biome::Ocean | Biome::DeepOcean | Biome::Shore | Biome::Lake
        )
    }

    pub fn color(&self) -> [u8; 4] {
        match self {
            Biome::DeepOcean => [0, 20, 80, 255],     // Very dark blue (no grey)
            Biome::Ocean => [5, 40, 120, 255],        // Dark ocean blue (more blue)
            Biome::Shore => [20, 70, 160, 255],       // Bright blue shallow water (vivid blue)
            Biome::Beach => [220, 200, 160, 255],     // Light brown/tan sand
            Biome::Plains => [120, 180, 90, 255],     // Light green grassland
            Biome::Forest => [50, 120, 50, 255],      // Forest green
            Biome::Hills => [140, 160, 100, 255],     // Brown-green
            Biome::Mountains => [140, 130, 120, 255], // Gray-brown
            Biome::SnowPeaks => [245, 245, 250, 255], // Snow white
            Biome::River => [20, 60, 120, 255],       // Dark river blue
            Biome::Lake => [15, 55, 100, 255],        // Dark lake blue
            Biome::Swamp => [60, 80, 60, 255],        // Swamp green-brown
            Biome::Desert => [230, 210, 170, 255],    // Desert sand (lighter than beach)
        }
    }

    pub fn elevation_color(elevation: f64) -> [u8; 4] {
        // Smooth gradient based on elevation
        let e = (elevation + 1.0) / 2.0; // Normalize to 0-1

        if e < 0.2 {
            // Deep water to shallow water - pure blues only
            let t = e / 0.2;
            let r = (0.0 + t * 20.0) as u8;
            let g = (20.0 + t * 50.0) as u8;
            let b = (80.0 + t * 80.0) as u8;
            [r, g, b, 255]
        } else if e < 0.45 {
            // Beach to plains
            let t = (e - 0.2) / 0.25;
            let r = (238.0 - t * 118.0) as u8;
            let g = (214.0 - t * 34.0) as u8;
            let b = (175.0 - t * 85.0) as u8;
            [r, g, b, 255]
        } else if e < 0.6 {
            // Plains to hills
            let t = (e - 0.45) / 0.15;
            let r = (120.0 + t * 20.0) as u8;
            let g = (180.0 - t * 20.0) as u8;
            let b = (90.0 + t * 10.0) as u8;
            [r, g, b, 255]
        } else if e < 0.85 {
            // Hills to mountains
            let t = (e - 0.6) / 0.25;
            let r = 140.0 as u8;
            let g = (160.0 - t * 30.0) as u8;
            let b = (100.0 + t * 20.0) as u8;
            [r, g, b, 255]
        } else {
            // Mountains to snow
            let t = (e - 0.85) / 0.15;
            let r = (140.0 + t * 105.0) as u8;
            let g = (130.0 + t * 115.0) as u8;
            let b = (120.0 + t * 130.0) as u8;
            [r, g, b, 255]
        }
    }
}

impl TerrainGenerator {
    /// Classify a tile. Elevation is histogram-equalized (its value is the
    /// area quantile), so each threshold below directly controls the share of
    /// water/land that biome covers.
    pub(super) fn determine_biome(&self, elevation: f64, moisture: f64, temperature: f64) -> Biome {
        if elevation < -0.45 {
            // Deepest 45% of water
            Biome::DeepOcean
        } else if elevation < -0.05 {
            Biome::Ocean
        } else if elevation < 0.0 {
            // Shallow water - the 5% of water nearest sea level
            Biome::Shore
        } else if elevation < 0.04 {
            // Beaches - lowest 4% of land
            Biome::Beach
        } else if elevation < 0.18 {
            // Coastal lowlands - varied terrain
            if moisture > 0.85 {
                // Swamps only in very wet areas (rare)
                Biome::Swamp
            } else if moisture > 0.55 {
                // Coastal forests common
                Biome::Forest
            } else if moisture < 0.25 && temperature > 0.7 {
                Biome::Desert
            } else {
                // Coastal grasslands/plains
                Biome::Plains
            }
        } else if elevation < 0.60 {
            // Lowland plains and forests
            if moisture > 0.8 && temperature < 0.5 {
                // Inland swamps (rare)
                Biome::Swamp
            } else if moisture > 0.5 {
                Biome::Forest
            } else if moisture < 0.3 && temperature > 0.6 {
                Biome::Desert
            } else {
                Biome::Plains
            }
        } else if elevation < 0.82 {
            // Hills: ~22% of land
            Biome::Hills
        } else if elevation < 0.95 {
            // Mountains: ~13% of land
            Biome::Mountains
        } else {
            // Snow peaks: highest 5% of land
            Biome::SnowPeaks
        }
    }
}

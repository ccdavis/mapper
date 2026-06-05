//! Procedural terrain generation.
//!
//! The generator is split into focused modules:
//! - [`elevation`]: continent shapes and the elevation field
//! - [`climate`]: moisture and temperature fields
//! - [`biome`]: biome classification and colors
//! - [`hydrology`]: river tracing
//! - [`settlements`]: city placement, road pathfinding, bridges
//! - [`labels`]: named-region detection and label placement
//! - [`names`]: procedural place-name generation

mod biome;
mod climate;
mod elevation;
mod hydrology;
mod labels;
mod names;
mod settlements;
mod types;

pub use biome::Biome;
pub use types::{Bridge, City, GenerationSettings, PlaceLabel, Road, TerrainMap, TerrainPoint};

use noise::Perlin;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

pub struct TerrainGenerator {
    elevation_noise: Perlin,
    moisture_noise: Perlin,
    temperature_noise: Perlin,
    detail_noise: Perlin,
    rng: ChaCha8Rng,
    settings: GenerationSettings,
}

impl TerrainGenerator {
    pub fn new(seed: u32) -> Self {
        Self::new_with_settings(seed, GenerationSettings::default())
    }

    pub fn new_with_settings(seed: u32, settings: GenerationSettings) -> Self {
        TerrainGenerator {
            elevation_noise: Perlin::new(seed),
            moisture_noise: Perlin::new(seed.wrapping_add(1)),
            temperature_noise: Perlin::new(seed.wrapping_add(2)),
            detail_noise: Perlin::new(seed.wrapping_add(3)),
            rng: ChaCha8Rng::seed_from_u64(seed as u64),
            settings,
        }
    }

    pub fn set_settings(&mut self, settings: GenerationSettings) {
        self.settings = settings;
    }

    pub fn generate(&mut self, width: usize, height: usize) -> TerrainMap {
        let mut terrain = vec![
            vec![
                TerrainPoint {
                    elevation: 0.0,
                    moisture: 0.0,
                    temperature: 0.0,
                    biome: Biome::Plains,
                };
                width
            ];
            height
        ];

        // Generate the elevation field first (sea level depends on the whole
        // distribution), then moisture (depends on distance to the ocean),
        // then temperature and biome for every tile
        let elevations = self.generate_elevation_field(width, height);
        let moistures = self.generate_moisture_field(&elevations);
        for y in 0..height {
            for x in 0..width {
                let elevation = elevations[y][x];
                let moisture = moistures[y][x];
                let temperature = self.generate_temperature(x, y, width, height, elevation);
                let biome = self.determine_biome(elevation, moisture, temperature);

                terrain[y][x] = TerrainPoint {
                    elevation,
                    moisture,
                    temperature,
                    biome,
                };
            }
        }

        // Generate rivers and lakes (lake tiles are marked in `terrain`)
        let rivers = self.generate_hydrology(&mut terrain);

        // Apply river erosion and widen rivers
        for river in &rivers {
            for &(x, y) in river {
                if x < width && y < height {
                    // Rivers pass through lakes without overwriting them
                    if terrain[y][x].biome != Biome::Lake {
                        terrain[y][x].biome = Biome::River;
                    }
                    terrain[y][x].elevation *= 0.9; // More erosion

                    // Widen rivers by affecting adjacent cells
                    for dy in -1i32..=1 {
                        for dx in -1i32..=1 {
                            // Direct neighbors get more effect
                            if dx != 0 && dy != 0 {
                                continue;
                            }
                            let nx = (x as i32 + dx) as usize;
                            let ny = (y as i32 + dy) as usize;
                            if nx < width && ny < height && terrain[ny][nx].elevation > -0.1 {
                                terrain[ny][nx].elevation *= 0.95;
                            }
                        }
                    }
                }
            }
        }

        // Generate cities following Zipf's law
        let cities = self.generate_cities(&terrain);

        // Generate roads connecting cities
        let (roads, bridges) = self.generate_roads(&terrain, &cities, &rivers);

        // Generate place labels including forests and swamps
        let labels = self.generate_labels(&terrain, &rivers);

        TerrainMap {
            width,
            height,
            terrain,
            labels,
            rivers,
            cities,
            roads,
            bridges,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn land_percentage_matches_settings() {
        for &land in &[0.2f32, 0.4, 0.7] {
            let settings = GenerationSettings {
                land_percentage: land,
                ..Default::default()
            };
            let mut generator = TerrainGenerator::new_with_settings(12345, settings);
            let map = generator.generate(160, 120);
            let land_tiles = map
                .terrain
                .iter()
                .flatten()
                .filter(|p| p.elevation > 0.0)
                .count();
            let fraction = land_tiles as f32 / (160.0 * 120.0);
            assert!(
                (fraction - land).abs() < 0.03,
                "target {} but generated {}",
                land,
                fraction
            );
        }
    }

    #[test]
    fn same_seed_generates_identical_maps() {
        let make = || TerrainGenerator::new(99).generate(120, 90);
        let a = make();
        let b = make();
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap(),
            "generation must be deterministic for a fixed seed"
        );
    }

    #[test]
    fn rivers_reach_water_edge_lake_or_confluence() {
        use std::collections::HashMap;

        let mut generator = TerrainGenerator::new(7);
        let map = generator.generate(160, 120);
        assert!(!map.rivers.is_empty(), "default settings should produce rivers");

        // How many rivers pass through each tile (for confluence detection)
        let mut coverage: HashMap<(usize, usize), usize> = HashMap::new();
        for river in &map.rivers {
            for &p in river {
                *coverage.entry(p).or_insert(0) += 1;
            }
        }

        for river in &map.rivers {
            let &(x, y) = river.last().unwrap();
            let end = &map.terrain[y][x];
            let at_edge = x == 0 || y == 0 || x >= map.width - 1 || y >= map.height - 1;
            let at_sea = end.elevation < 0.0;
            let in_lake = end.biome == Biome::Lake;
            let confluence = coverage[&(x, y)] >= 2;
            assert!(
                at_sea || at_edge || in_lake || confluence,
                "river ends inland at ({}, {})",
                x,
                y
            );
        }
    }
}

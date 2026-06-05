use serde::{Deserialize, Serialize};

use super::biome::Biome;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainPoint {
    pub elevation: f64,   // -1.0 to 1.0
    pub moisture: f64,    // 0.0 to 1.0
    pub temperature: f64, // 0.0 to 1.0
    pub biome: Biome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceLabel {
    pub x: f32,
    pub y: f32,
    pub name: String,
    pub feature_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct City {
    pub x: usize,
    pub y: usize,
    pub name: String,
    pub population: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Road {
    pub path: Vec<(usize, usize)>,
    pub name: String,
    pub road_type: String,    // "highway", "road", "trail"
    pub bridges: Vec<Bridge>, // Bridges along this road
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bridge {
    pub x: usize,
    pub y: usize,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TerrainMap {
    pub width: usize,
    pub height: usize,
    pub terrain: Vec<Vec<TerrainPoint>>,
    pub labels: Vec<PlaceLabel>,
    pub rivers: Vec<Vec<(usize, usize)>>,
    pub cities: Vec<City>,
    pub roads: Vec<Road>,
    pub bridges: Vec<Bridge>,
}

#[derive(Debug, Clone, Copy)]
pub struct GenerationSettings {
    pub river_density: f32,   // 0.0 (low) to 1.0 (high)
    pub city_density: f32,    // 0.0 (low) to 1.0 (high)
    pub land_percentage: f32, // 0.0 (mostly water) to 1.0 (mostly land)
}

impl Default for GenerationSettings {
    fn default() -> Self {
        GenerationSettings {
            river_density: 0.5,   // medium
            city_density: 0.5,    // medium
            land_percentage: 0.4, // 40% land, 60% water
        }
    }
}

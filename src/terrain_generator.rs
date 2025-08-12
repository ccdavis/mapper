use noise::{NoiseFn, Perlin};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainPoint {
    pub elevation: f64,      // -1.0 to 1.0
    pub moisture: f64,       // 0.0 to 1.0
    pub temperature: f64,    // 0.0 to 1.0
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
    pub road_type: String, // "highway", "road", "trail"
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
    pub river_density: f32,  // 0.0 (low) to 1.0 (high)
    pub city_density: f32,   // 0.0 (low) to 1.0 (high)
    pub land_percentage: f32, // 0.0 (mostly water) to 1.0 (mostly land)
}

impl Default for GenerationSettings {
    fn default() -> Self {
        GenerationSettings {
            river_density: 0.5,    // medium
            city_density: 0.5,     // medium
            land_percentage: 0.4,  // 40% land, 60% water
        }
    }
}

pub struct TerrainGenerator {
    elevation_noise: Perlin,
    moisture_noise: Perlin,
    temperature_noise: Perlin,
    detail_noise: Perlin,
    continent_noise: Perlin,
    rng: ChaCha8Rng,
    settings: GenerationSettings,
}

impl TerrainGenerator {
    pub fn new(seed: u32) -> Self {
        Self::new_with_settings(seed, GenerationSettings::default())
    }
    
    pub fn new_with_settings(seed: u32, settings: GenerationSettings) -> Self {
        let elevation_noise = Perlin::new(seed);
        let moisture_noise = Perlin::new(seed + 1);
        let temperature_noise = Perlin::new(seed + 2);
        let detail_noise = Perlin::new(seed + 3);
        let continent_noise = Perlin::new(seed + 4);
        let rng = ChaCha8Rng::seed_from_u64(seed as u64);
        
        TerrainGenerator {
            elevation_noise,
            moisture_noise,
            temperature_noise,
            detail_noise,
            continent_noise,
            rng,
            settings,
        }
    }
    
    pub fn set_settings(&mut self, settings: GenerationSettings) {
        self.settings = settings;
    }
    
    pub fn generate(&mut self, width: usize, height: usize) -> TerrainMap {
        let mut terrain = vec![vec![TerrainPoint {
            elevation: 0.0,
            moisture: 0.0,
            temperature: 0.0,
            biome: Biome::Plains,
        }; width]; height];
        
        // Generate elevation map using fractal noise
        for y in 0..height {
            for x in 0..width {
                let elevation = self.generate_elevation(x, y, width, height);
                let moisture = self.generate_moisture(x, y, width, height);
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
        
        // Generate rivers
        let rivers = self.generate_rivers(&terrain);
        
        // Apply river erosion and widen rivers
        for river in &rivers {
            for &(x, y) in river {
                if x < width && y < height {
                    terrain[y][x].biome = Biome::River;
                    terrain[y][x].elevation *= 0.9; // More erosion
                    
                    // Widen rivers by affecting adjacent cells
                    for dy in -1i32..=1 {
                        for dx in -1i32..=1 {
                            let nx = (x as i32 + dx) as usize;
                            let ny = (y as i32 + dy) as usize;
                            if nx < width && ny < height && terrain[ny][nx].elevation > -0.1 {
                                if dx == 0 || dy == 0 { // Direct neighbors get more effect
                                    terrain[ny][nx].elevation *= 0.95;
                                }
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
        let labels = self.generate_labels(&terrain, &rivers, &cities);
        
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
    
    fn generate_elevation(&self, x: usize, y: usize, width: usize, height: usize) -> f64 {
        // Normalize coordinates to [0, 1]
        let nx = x as f64 / width as f64;
        let ny = y as f64 / height as f64;
        
        // Scale factor for noise frequencies based on map size
        // Assuming 160x120 as baseline, scale proportionally
        let freq_scale = (width as f64 / 160.0).min(height as f64 / 120.0);
        
        let land_target = self.settings.land_percentage as f64;
        
        // Decide formation type based on seed
        let formation_seed = self.continent_noise.get([0.777, 0.333]) * 100.0;
        let formation_type = formation_seed.abs() as i32 % 5;
        
        // Decide if this map should have continents reaching edges (25% chance)
        let seed_hash = (self.continent_noise.get([0.123, 0.456]) * 1000.0) as i32;
        let has_edge_continent = (seed_hash.abs() % 100) < 25;
        
        let mut continent_value: f64 = -1.0; // Start with ocean
        
        // Different continent generation strategies
        match formation_type {
            0 => {
                // VOLCANIC ISLAND CHAIN - following tectonic lines like Hawaii or Indonesia
                let chain_angle = self.continent_noise.get([0.5, 0.5]) * std::f64::consts::TAU;
                let chain_length = 0.6 + land_target * 0.3;
                let num_islands = 4 + (land_target * 6.0) as i32;
                
                // Start position of the chain
                let start_x = 0.5 + self.continent_noise.get([1.5, 1.5]) * 0.3;
                let start_y = 0.5 + self.continent_noise.get([2.5, 2.5]) * 0.3;
                
                for i in 0..num_islands {
                    let t = i as f64 / num_islands as f64;
                    
                    // Islands follow a curved line (subduction zone)
                    let curve = (t * std::f64::consts::PI).sin() * 0.1;
                    let ix = start_x + chain_angle.cos() * t * chain_length + chain_angle.sin() * curve;
                    let iy = start_y + chain_angle.sin() * t * chain_length - chain_angle.cos() * curve;
                    
                    // Add some scatter
                    let scatter_x = self.continent_noise.get([i as f64 * 3.0, i as f64 * 4.0]) * 0.03;
                    let scatter_y = self.continent_noise.get([i as f64 * 4.0, i as f64 * 3.0]) * 0.03;
                    
                    let dx = nx - (ix + scatter_x);
                    let dy = ny - (iy + scatter_y);
                    let dist = (dx * dx + dy * dy).sqrt();
                    
                    // Varied island sizes - larger in middle of chain
                    let size_factor = 1.0 - (t - 0.5).abs() * 0.5;
                    let island_size = (0.03 + size_factor * 0.05) * (1.0 + land_target * 0.5);
                    
                    if dist < island_size {
                        // Volcanic profile - steep but not too tall
                        let height = (1.0 - dist / island_size).powf(1.5) * 0.8;
                        continent_value = continent_value.max(height);
                    }
                }
            },
            1 => {
                // TECTONIC RIDGE - elongated continent with mountain spine
                let ridge_angle = self.continent_noise.get([1.0, 2.0]) * std::f64::consts::TAU; // Full rotation
                let ridge_length = 0.3 + land_target * 0.2;  // Reduced length
                let ridge_width = 0.2 + land_target * 0.15;   // Increased width for less elongation
                
                // Center can be offset
                let cx = 0.5 + self.continent_noise.get([3.0, 4.0]) * 0.2;
                let cy = 0.5 + self.continent_noise.get([4.0, 3.0]) * 0.2;
                
                // Rotate coordinates around center
                let dx = nx - cx;
                let dy = ny - cy;
                let rotated_x = dx * ridge_angle.cos() - dy * ridge_angle.sin();
                let rotated_y = dx * ridge_angle.sin() + dy * ridge_angle.cos();
                
                // Check if point is within ridge bounds
                if rotated_x.abs() < ridge_length && rotated_y.abs() < ridge_width {
                    // Higher elevation along the spine
                    let spine_distance = rotated_y.abs() / ridge_width;
                    let along_ridge = rotated_x.abs() / ridge_length;
                    
                    // Mountains along the spine, lower at edges
                    let base_height = (1.0 - spine_distance) * 0.8;
                    let ridge_variation = self.elevation_noise.get([nx * 20.0 * freq_scale, ny * 20.0 * freq_scale]) * 0.3;
                    let taper = 1.0 - along_ridge * 0.5; // Taper towards ends
                    
                    continent_value = continent_value.max(base_height * taper + ridge_variation);
                }
            },
            2 => {
                // CRESCENT/ARC - like Japan or Indonesia
                let arc_center_x = 0.5 + self.continent_noise.get([5.0, 6.0]) * 0.3;
                let arc_center_y = 0.5 + self.continent_noise.get([6.0, 5.0]) * 0.3;
                let arc_radius = 0.3 + land_target * 0.2;
                let arc_width = 0.08 + land_target * 0.1;
                
                // Start and end angles for the arc
                let start_angle = self.continent_noise.get([7.0, 8.0]) * std::f64::consts::PI;
                let arc_span = std::f64::consts::PI * (0.5 + land_target * 0.5);
                
                let dx = nx - arc_center_x;
                let dy = ny - arc_center_y;
                let dist = (dx * dx + dy * dy).sqrt();
                let angle = dy.atan2(dx);
                
                // Normalize angle relative to start
                let mut angle_diff = angle - start_angle;
                while angle_diff < 0.0 { angle_diff += std::f64::consts::TAU; }
                while angle_diff > std::f64::consts::TAU { angle_diff -= std::f64::consts::TAU; }
                
                // Check if within arc
                if angle_diff < arc_span && (dist - arc_radius).abs() < arc_width {
                    let radial_factor = 1.0 - ((dist - arc_radius).abs() / arc_width);
                    let height = radial_factor * 0.7 + self.elevation_noise.get([nx * 15.0 * freq_scale, ny * 15.0 * freq_scale]) * 0.3;
                    continent_value = continent_value.max(height);
                }
            },
            3 => {
                // MULTI-PLATE CONTINENT - multiple tectonic plates forming a complex landmass
                // Like Africa-Europe or Asia with multiple geological centers
                
                let num_plates = 2 + (land_target * 2.0) as i32;
                
                for p in 0..num_plates {
                    let plate_offset = p as f64 * 50.0;
                    
                    // Each plate has its own center and characteristics
                    let px = 0.5 + self.continent_noise.get([plate_offset * 0.3, plate_offset * 0.4]) * 0.4;
                    let py = 0.5 + self.continent_noise.get([plate_offset * 0.4, plate_offset * 0.3]) * 0.4;
                    
                    // Plate size varies
                    let plate_size = 0.2 + self.continent_noise.get([plate_offset * 0.5, plate_offset * 0.6]).abs() * 0.2;
                    
                    let dx = nx - px;
                    let dy = ny - py;
                    let dist = (dx * dx + dy * dy).sqrt();
                    
                    // Use noise to create irregular plate boundaries
                    let boundary_noise = self.elevation_noise.get([nx * 8.0 * freq_scale, ny * 8.0 * freq_scale]) * 0.15
                                       + self.detail_noise.get([nx * 15.0 * freq_scale, ny * 15.0 * freq_scale]) * 0.1;
                    
                    let effective_size = plate_size + boundary_noise;
                    
                    if dist < effective_size {
                        // Height varies across the plate
                        let plate_height = (1.0 - dist / effective_size) * 0.5;
                        
                        // Add rifts and mountain ranges where plates meet
                        let rift_noise = self.continent_noise.get([nx * 20.0 * freq_scale, ny * 20.0 * freq_scale]);
                        
                        // Check if we're near another plate (collision zone)
                        let mut near_other_plate = false;
                        for other_p in 0..num_plates {
                            if other_p != p {
                                let other_offset = other_p as f64 * 50.0;
                                let opx = 0.5 + self.continent_noise.get([other_offset * 0.3, other_offset * 0.4]) * 0.4;
                                let opy = 0.5 + self.continent_noise.get([other_offset * 0.4, other_offset * 0.3]) * 0.4;
                                let odist = ((nx - opx).powi(2) + (ny - opy).powi(2)).sqrt();
                                let other_size = 0.2 + self.continent_noise.get([other_offset * 0.5, other_offset * 0.6]).abs() * 0.2;
                                
                                if odist < other_size + 0.05 {
                                    near_other_plate = true;
                                    break;
                                }
                            }
                        }
                        
                        let height = if near_other_plate && rift_noise > 0.2 {
                            // Mountain range at plate boundary (reduced from before)
                            plate_height + 0.3
                        } else if rift_noise < -0.3 {
                            // Rift valley
                            plate_height * 0.5
                        } else {
                            plate_height
                        };
                        
                        continent_value = continent_value.max(height);
                    }
                }
                
                // Add isthmuses connecting plates
                if land_target > 0.3 {
                    let isthmus_noise = self.elevation_noise.get([nx * 6.0 * freq_scale, ny * 6.0 * freq_scale]);
                    if isthmus_noise > 0.4 && continent_value > -0.5 {
                        // Create thin land bridges
                        continent_value = continent_value.max(0.3);
                    }
                }
            },
            _ => {
                // COMPLEX ARCHIPELAGO - more natural distribution
                // Use power law for realistic island size distribution
                
                // Create island clusters with fractal distribution
                let num_major_clusters = 1 + (land_target * 2.0) as i32;
                
                for c in 0..num_major_clusters {
                    let cluster_seed = c as f64 * 47.3;
                    
                    // Cluster center with some randomness
                    let cx = 0.5 + self.continent_noise.get([cluster_seed * 0.13, cluster_seed * 0.17]) * 0.4;
                    let cy = 0.5 + self.continent_noise.get([cluster_seed * 0.19, cluster_seed * 0.11]) * 0.4;
                    
                    // Power law distribution for island count (many small, few large)
                    let base_islands = 3 + (land_target * 5.0) as i32;
                    
                    for i in 0..base_islands {
                        let island_seed = i as f64 * 13.7 + cluster_seed;
                        
                        // Use log-normal distribution for island sizes
                        let size_factor = (-((i as f64) / 3.0)).exp(); // Exponential decay
                        let size_variation = self.elevation_noise.get([island_seed * 0.23, island_seed * 0.29]).abs();
                        
                        // Islands cluster fractally - some tight groups, some scattered
                        let scatter = size_factor * 0.2 + 0.05;
                        let angle = island_seed * 2.3; // Avoid grid patterns
                        let radius = scatter * (1.0 + size_variation);
                        
                        let ix = cx + angle.cos() * radius + self.continent_noise.get([island_seed * 0.31, island_seed * 0.37]) * scatter;
                        let iy = cy + angle.sin() * radius + self.continent_noise.get([island_seed * 0.41, island_seed * 0.43]) * scatter;
                        
                        let dx = nx - ix;
                        let dy = ny - iy;
                        
                        // Non-circular island shapes
                        let shape_distortion = self.detail_noise.get([nx * 50.0 * freq_scale, ny * 50.0 * freq_scale]) * 0.3;
                        let effective_dist = ((dx * dx * (1.0 + shape_distortion)) + (dy * dy * (1.0 - shape_distortion))).sqrt();
                        
                        // Log-scale island sizes
                        let island_size = (0.02 + size_factor * 0.1) * (1.0 + size_variation * 0.5) * land_target.sqrt();
                        
                        if effective_dist < island_size {
                            // Varied elevation profiles
                            let profile_power = 1.5 + size_variation; // Flatter or steeper
                            let height = (1.0 - effective_dist / island_size).powf(profile_power) * (0.4 + size_factor * 0.4);
                            continent_value = continent_value.max(height);
                        }
                    }
                }
                
                // Add underwater ridges connecting some islands
                let ridge_noise = self.elevation_noise.get([nx * 5.0 * freq_scale, ny * 5.0 * freq_scale]);
                if ridge_noise > 0.3 && continent_value > -0.8 {
                    // Shallow areas between islands
                    continent_value = continent_value.max(-0.1 + ridge_noise * 0.3);
                }
            }
        }
        
        // Add more natural island distribution using multiple scales
        if land_target > 0.2 {
            // Use log scale for more natural size variation (many small, few large)
            let large_scale = self.continent_noise.get([nx * 3.0 * freq_scale, ny * 3.0 * freq_scale]);
            let medium_scale = self.elevation_noise.get([nx * 8.0 * freq_scale, ny * 8.0 * freq_scale]);
            let small_scale = self.detail_noise.get([nx * 25.0 * freq_scale, ny * 25.0 * freq_scale]);
            
            // Create clustered distribution - islands near other islands
            let cluster_factor = (large_scale * 0.5 + 0.5).powf(2.0); // Squared for clustering
            
            // Combine scales with log-normal distribution for natural size variation
            let island_noise = large_scale * 0.4 + medium_scale * 0.4 * cluster_factor + small_scale * 0.2;
            
            // Use different thresholds for different island sizes (log scale)
            if island_noise > 0.6 {
                // Large islands (rare)
                continent_value = continent_value.max(island_noise * 0.8);
            } else if island_noise > 0.45 && cluster_factor > 0.3 {
                // Medium islands (clustered)
                continent_value = continent_value.max(island_noise * 0.5);
            } else if island_noise > 0.35 && self.detail_noise.get([nx * 100.0, ny * 100.0]) > (0.5 - 0.3 * land_target) {
                // Small islands (common, but randomly distributed)
                continent_value = continent_value.max(island_noise * 0.3);
            }
        }
        
        // Edge falloff only for certain formation types that should be islands
        // Allow volcanic islands and irregular blobs to have edge falloff, but not others
        if formation_type == 0 || (formation_type == 3 && !has_edge_continent) {
            let edge_dist = (0.5 - (nx - 0.5).abs()).min(0.5 - (ny - 0.5).abs());
            if edge_dist < 0.05 {  // Reduced from 0.1 for more natural edges
                continent_value = continent_value * (edge_dist / 0.05) - (1.0 - edge_dist / 0.05);
            }
        }
        
        // Add coastline detail with rotation to avoid horizontal bias
        let angle = self.continent_noise.get([nx * 2.0, ny * 2.0]) * std::f64::consts::PI;
        let rotated_nx = nx * angle.cos() - ny * angle.sin();
        let rotated_ny = nx * angle.sin() + ny * angle.cos();
        
        let coastline = self.elevation_noise.get([rotated_nx * 40.0 * freq_scale, rotated_ny * 40.0 * freq_scale]) * 0.1
                      + self.detail_noise.get([nx * 80.0 * freq_scale, ny * 80.0 * freq_scale]) * 0.05;
        
        let base_elevation = continent_value + coastline;
        
        // Sea level determines land percentage
        let sea_level = -0.1 + (1.0 - land_target) * 0.3;
        
        // Final elevation with clear land/water boundary
        let elevation = if base_elevation > sea_level {
            ((base_elevation - sea_level) / (1.0 - sea_level)).min(1.0).max(0.01)
        } else {
            ((base_elevation - sea_level) / (1.0 + sea_level)).max(-1.0)
        };
        
        elevation
    }
    
    fn generate_moisture(&self, x: usize, y: usize, width: usize, height: usize) -> f64 {
        let scale = 1.0 / width.min(height) as f64;
        let nx = x as f64 * scale;
        let ny = y as f64 * scale;
        
        let moisture = self.moisture_noise.get([nx * 3.0, ny * 3.0]) * 0.5 + 0.5;
        moisture.max(0.0).min(1.0)
    }
    
    fn generate_temperature(&self, x: usize, y: usize, width: usize, height: usize, elevation: f64) -> f64 {
        let scale = 1.0 / width.min(height) as f64;
        let nx = x as f64 * scale;
        let ny = y as f64 * scale;
        
        // Temperature decreases with elevation and latitude
        let base_temp = self.temperature_noise.get([nx * 2.0, ny * 2.0]) * 0.5 + 0.5;
        let latitude_factor = (y as f64 / height as f64 - 0.5).abs() * 2.0;
        let elevation_factor = (elevation + 1.0) / 2.0;
        
        let temperature = base_temp * (1.0 - latitude_factor * 0.3) * (1.0 - elevation_factor * 0.4);
        temperature.max(0.0).min(1.0)
    }
    
    fn determine_biome(&self, elevation: f64, moisture: f64, temperature: f64) -> Biome {
        if elevation < -0.4 {
            Biome::DeepOcean
        } else if elevation < -0.15 {
            Biome::Ocean
        } else if elevation < -0.05 {
            // Shallow water
            Biome::Shore
        } else if elevation < 0.05 {
            // Beaches - coastal sand
            Biome::Beach
        } else if elevation < 0.15 {
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
        } else if elevation < 0.25 {
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
        } else if elevation < 0.5 {
            // More hills, less mountains
            Biome::Hills
        } else if elevation < 0.75 {
            // Mountains only at higher elevations
            Biome::Mountains
        } else {
            // Snow peaks only at very high elevations
            Biome::SnowPeaks
        }
    }
    
    fn generate_rivers(&mut self, terrain: &Vec<Vec<TerrainPoint>>) -> Vec<Vec<(usize, usize)>> {
        let mut rivers = Vec::new();
        
        // Handle 0% case - no rivers at all
        if self.settings.river_density < 0.01 {
            return rivers;
        }
        
        // Scale river count based on settings (0.0 = 0 rivers, 0.5 = 10-20 rivers, 1.0 = 30-50 rivers)
        let min_rivers = (self.settings.river_density * 20.0) as usize;
        let max_rivers = (self.settings.river_density * 50.0) as usize;
        let num_rivers = if min_rivers == max_rivers {
            min_rivers
        } else {
            self.rng.gen_range(min_rivers..=max_rivers)
        };
        
        for _ in 0..num_rivers {
            // Start from mountain/hill areas
            let mut start_x = 0;
            let mut start_y = 0;
            let mut found_start = false;
            
            for _ in 0..200 {
                let x = self.rng.gen_range(0..terrain[0].len());
                let y = self.rng.gen_range(0..terrain.len());
                
                // Start rivers from any elevated land (lowered threshold for islands)
                if terrain[y][x].elevation > 0.15 && terrain[y][x].elevation < 0.85 {  
                    start_x = x;
                    start_y = y;
                    found_start = true;
                    break;
                }
            }
            
            if !found_start {
                continue;
            }
            
            let mut river = Vec::new();
            let mut x = start_x;
            let mut y = start_y;
            let mut visited = HashMap::new();
            
            // Flow downhill
            for _ in 0..200 {
                river.push((x, y));
                visited.insert((x, y), true);
                
                let current_elevation = terrain[y][x].elevation;
                
                // Check if we reached ocean/sea
                if current_elevation < -0.05 {
                    // Successfully reached the sea!
                    if river.len() > 10 {  // Only keep rivers that are long enough
                        rivers.push(river);
                    }
                    break;
                }
                
                // Find lowest neighbor
                let mut lowest_elevation = current_elevation;
                let mut next_x = x;
                let mut next_y = y;
                
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        
                        if nx >= 0 && nx < terrain[0].len() as i32 && 
                           ny >= 0 && ny < terrain.len() as i32 {
                            let nx = nx as usize;
                            let ny = ny as usize;
                            
                            if !visited.contains_key(&(nx, ny)) && terrain[ny][nx].elevation < lowest_elevation {
                                lowest_elevation = terrain[ny][nx].elevation;
                                next_x = nx;
                                next_y = ny;
                            }
                        }
                    }
                }
                
                if next_x == x && next_y == y {
                    // No lower point found - river ends (forms a lake or disappears)
                    // Only add river if it's substantial (reduced minimum for islands)
                    if river.len() > 8 && current_elevation < 0.2 {
                        rivers.push(river);
                    }
                    break;
                }
                
                x = next_x;
                y = next_y;
            }
        }
        
        rivers
    }
    
    fn generate_cities(&mut self, terrain: &Vec<Vec<TerrainPoint>>) -> Vec<City> {
        let mut cities = Vec::new();
        
        // Handle 0% case - no cities at all
        if self.settings.city_density < 0.01 {
            return cities;
        }
        
        // First, find all valid land tiles for city placement
        let mut valid_positions = Vec::new();
        for y in 2..terrain.len()-2 {  // Reduced margin for islands
            for x in 2..terrain[0].len()-2 {
                let point = &terrain[y][x];
                // Cities can be on any stable land biome
                if matches!(point.biome, 
                    Biome::Plains | Biome::Hills | Biome::Forest | 
                    Biome::Desert | Biome::Beach) {
                    // For islands, allow cities closer to water (coastal cities are common)
                    // Only check for immediate water, not 2 tiles away
                    let mut is_valid = true;
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            let nx = (x as i32 + dx) as usize;
                            let ny = (y as i32 + dy) as usize;
                            if nx < terrain[0].len() && ny < terrain.len() {
                                // Don't place city directly in water
                                if dx == 0 && dy == 0 && matches!(terrain[ny][nx].biome, 
                                    Biome::Ocean | Biome::DeepOcean | Biome::Lake | Biome::Shore) {
                                    is_valid = false;
                                    break;
                                }
                            }
                        }
                        if !is_valid { break; }
                    }
                    // Add position if it's valid land
                    if is_valid {
                        valid_positions.push((x, y));
                    }
                }
            }
        }
        
        if valid_positions.is_empty() {
            return cities;
        }
        
        // Scale city counts based on settings and available land
        let land_factor = valid_positions.len() as f32 / (terrain.len() * terrain[0].len()) as f32;
        
        // Major cities: 0-10 based on density and available land
        let num_major_cities = if self.settings.city_density < 0.1 {
            0
        } else {
            let base = ((self.settings.city_density - 0.1) * 10.0 * land_factor * 2.0) as usize;
            base.min(10)
        };
        
        // Medium cities: 0-25 based on density
        let num_medium_cities = if self.settings.city_density < 0.05 {
            0
        } else {
            let base = ((self.settings.city_density - 0.05) * 25.0 * land_factor * 2.0) as usize;
            base.min(25)
        };
        
        // Small towns: 0-70 based on density
        let num_towns = {
            let base = (self.settings.city_density * 70.0 * land_factor * 2.0) as usize;
            base.min(70)
        };
        
        let mut placed_positions = Vec::new();
        
        // Generate city populations following Zipf's law for major cities
        let base_population = 500000;
        let mut populations: Vec<u32> = Vec::new();
        
        // Major cities
        for i in 1..=num_major_cities {
            populations.push((base_population as f64 / i as f64) as u32);
        }
        // Medium cities
        for _ in 0..num_medium_cities {
            populations.push(self.rng.gen_range(50000..150000));
        }
        // Small towns
        for _ in 0..num_towns {
            populations.push(self.rng.gen_range(5000..30000));
        }
        
        // Place cities on suitable terrain with better spacing
        for (idx, pop) in populations.iter().enumerate() {
            let mut attempts = 0;
            let is_major = idx < num_major_cities;
            let is_medium = idx < (num_major_cities + num_medium_cities);
            
            while attempts < 150 && !valid_positions.is_empty() {
                // Pick from valid land positions
                let pos_idx = self.rng.gen_range(0..valid_positions.len());
                let (x, y) = valid_positions[pos_idx];
                
                // We're already using valid land positions, so just get the terrain
                let point = &terrain[y][x];
                
                // Cities prefer certain terrain types
                let suitable = match point.biome {
                    Biome::Plains => true,
                    Biome::Beach | Biome::Shore => is_major || self.rng.gen_bool(0.7), // Major cities like coasts
                    Biome::Hills => self.rng.gen_bool(0.5),
                    Biome::Forest => self.rng.gen_bool(0.2),
                    _ => false,
                };
                
                if !suitable {
                    attempts += 1;
                    continue;
                }
                
                // Much larger minimum distances for better distribution
                let mut min_dist = if is_major { 
                    100.0  // Major cities need LOTS of space
                } else if is_medium { 
                    60.0   // Medium cities need good spacing
                } else { 
                    40.0   // Towns should also be well-spaced
                };
                
                // Check for grid alignment and minimum distances
                let mut too_close = false;
                let mut grid_aligned = false;
                
                for (i, &(cx, cy)) in placed_positions.iter().enumerate() {
                    let dx = x as f64 - cx as f64;
                    let dy = y as f64 - cy as f64;
                    let dist = (dx * dx + dy * dy).sqrt();
                    
                    // Prevent cities from lining up on same latitude/longitude
                    if (dx.abs() < 3.0 || dy.abs() < 3.0) && dist < 40.0 {
                        grid_aligned = true;  // Too aligned with existing city
                        break;
                    }
                    
                    // Special case: allow 1-2 towns near major cities (suburbs)
                    if !is_major && !is_medium && i < num_major_cities {
                        // Towns can be closer to major cities (suburbs)
                        if dist < 6.0 {
                            too_close = true; // But not too close
                            break;
                        } else if dist < 12.0 && self.rng.gen_bool(0.3) {
                            // 30% chance to allow suburb placement
                            min_dist = 8.0;
                        }
                    }
                    
                    if dist < min_dist {
                        too_close = true;
                        break;
                    }
                }
                
                // Add some offset to prevent grid patterns
                if grid_aligned && attempts < 100 {
                    attempts += 1;
                    continue;
                }
                
                if !too_close {
                    cities.push(City {
                        x,
                        y,
                        name: self.generate_city_name(cities.len()),
                        population: *pop,
                    });
                    placed_positions.push((x, y));
                    break;
                }
                attempts += 1;
            }
        }
        
        cities
    }
    
    fn generate_roads(&mut self, terrain: &Vec<Vec<TerrainPoint>>, cities: &Vec<City>, rivers: &Vec<Vec<(usize, usize)>>) -> (Vec<Road>, Vec<Bridge>) {
        let mut roads = Vec::new();
        let mut all_bridges = Vec::new();
        
        if cities.is_empty() {
            return (roads, all_bridges);
        }
        
        // Create a set of river points for quick lookup
        let mut river_points = std::collections::HashSet::new();
        for river in rivers {
            for &point in river {
                river_points.insert(point);
            }
        }
        
        // Track which cities are connected and existing road points for reuse
        let mut connected_cities = vec![false; cities.len()];
        let mut road_network: std::collections::HashMap<(usize, usize), Vec<usize>> = std::collections::HashMap::new();
        
        // Step 1: Create a minimum spanning tree for major cities to avoid parallel roads
        let major_count = cities.len().min(8);
        let mut mst_edges = Vec::new();
        
        if major_count > 1 {
            // Calculate all distances between major cities
            let mut edges = Vec::new();
            for i in 0..major_count {
                for j in i+1..major_count {
                    let dx = cities[i].x as f64 - cities[j].x as f64;
                    let dy = cities[i].y as f64 - cities[j].y as f64;
                    let dist = (dx * dx + dy * dy).sqrt();
                    edges.push((dist, i, j));
                }
            }
            edges.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            
            // Build MST using Kruskal's algorithm
            let mut union_find = (0..major_count).collect::<Vec<_>>();
            let find = |uf: &mut Vec<usize>, mut x: usize| -> usize {
                while uf[x] != x {
                    x = uf[x];
                }
                x
            };
            
            for (dist, i, j) in edges {
                if dist > 80.0 { break; }  // Don't connect very distant cities
                
                let root_i = find(&mut union_find, i);
                let root_j = find(&mut union_find, j);
                
                if root_i != root_j {
                    union_find[root_i] = root_j;
                    mst_edges.push((i, j));
                }
            }
        }
        
        // Step 2: Build main highways along MST edges
        for (i, j) in mst_edges {
            let path = self.find_path(terrain, cities[i].x, cities[i].y, cities[j].x, cities[j].y);
            if !path.is_empty() {
                connected_cities[i] = true;
                connected_cities[j] = true;
                
                // Store road segments for potential reuse
                for &point in &path {
                    road_network.entry(point)
                        .or_insert_with(Vec::new)
                        .push(roads.len());
                }
                
                // Detect bridges
                let mut bridges = Vec::new();
                for &(x, y) in &path {
                    if river_points.contains(&(x, y)) || 
                       (terrain[y][x].biome == Biome::River && !matches!(terrain[y][x].biome, Biome::Ocean | Biome::DeepOcean | Biome::Lake)) {
                        let bridge = Bridge {
                            x,
                            y,
                            name: self.generate_bridge_name(all_bridges.len()),
                        };
                        bridges.push(bridge.clone());
                        all_bridges.push(bridge);
                    }
                }
                
                roads.push(Road {
                    path,
                    name: format!("{} Highway", self.generate_road_name(roads.len())),
                    road_type: "highway".to_string(),
                    bridges,
                });
            }
        }
        
        // Step 3: Connect remaining cities, trying to create Y-junctions by connecting to existing roads
        for i in 0..cities.len() {
            if !connected_cities[i] {
                // Try to find the nearest point on an existing road
                let mut best_connection = None;
                let mut min_cost = f64::MAX;
                
                // First check if we can connect to an existing road network
                if !road_network.is_empty() {
                    for (&(rx, ry), _) in road_network.iter() {
                        let dx = cities[i].x as f64 - rx as f64;
                        let dy = cities[i].y as f64 - ry as f64;
                        let dist = (dx * dx + dy * dy).sqrt();
                        
                        // Only consider reasonably close road points
                        if dist < 30.0 && dist < min_cost {
                            min_cost = dist;
                            best_connection = Some((rx, ry, true));  // true = connect to road
                        }
                    }
                }
                
                // If no good road connection, find nearest connected city
                if best_connection.is_none() {
                    for j in 0..cities.len() {
                        if i != j && connected_cities[j] {
                            let dx = cities[i].x as f64 - cities[j].x as f64;
                            let dy = cities[i].y as f64 - cities[j].y as f64;
                            let dist = (dx * dx + dy * dy).sqrt();
                            
                            if dist < min_cost {
                                min_cost = dist;
                                best_connection = Some((cities[j].x, cities[j].y, false));  // false = connect to city
                            }
                        }
                    }
                }
                
                // If still no connection, connect to nearest city regardless
                if best_connection.is_none() {
                    let mut nearest = 0;
                    let mut min_dist = f64::MAX;
                    for j in 0..cities.len() {
                        if i != j {
                            let dx = cities[i].x as f64 - cities[j].x as f64;
                            let dy = cities[i].y as f64 - cities[j].y as f64;
                            let dist = (dx * dx + dy * dy).sqrt();
                            if dist < min_dist {
                                min_dist = dist;
                                nearest = j;
                            }
                        }
                    }
                    best_connection = Some((cities[nearest].x, cities[nearest].y, false));
                }
                
                if let Some((target_x, target_y, is_road_junction)) = best_connection {
                    let path = self.find_path(terrain, cities[i].x, cities[i].y, target_x, target_y);
                    if !path.is_empty() {
                        connected_cities[i] = true;
                        
                        // Store new road segments
                        for &point in &path {
                            road_network.entry(point)
                                .or_insert_with(Vec::new)
                                .push(roads.len());
                        }
                        
                        // Detect bridges
                        let mut bridges = Vec::new();
                        for &(x, y) in &path {
                            if river_points.contains(&(x, y)) || 
                               (terrain[y][x].biome == Biome::River && !matches!(terrain[y][x].biome, Biome::Ocean | Biome::DeepOcean | Biome::Lake)) {
                                let bridge = Bridge {
                                    x,
                                    y,
                                    name: self.generate_bridge_name(all_bridges.len()),
                                };
                                bridges.push(bridge.clone());
                                all_bridges.push(bridge);
                            }
                        }
                        
                        let road_type = if cities[i].population > 100000 {
                            "road"
                        } else {
                            "trail"
                        };
                        
                        let road_name = if is_road_junction {
                            format!("{} Branch", self.generate_road_name(roads.len()))
                        } else {
                            format!("{} {}", self.generate_road_name(roads.len()), 
                                   if road_type == "trail" { "Trail" } else { "Road" })
                        };
                        
                        roads.push(Road {
                            path,
                            name: road_name,
                            road_type: road_type.to_string(),
                            bridges,
                        });
                    }
                }
            }
        }
        
        // Add some partial roads from cities that just go into the wilderness
        for i in 0..cities.len() {
            if self.rng.gen_bool(0.3) { // 30% chance for each city to have an extra road
                // Pick a random direction and distance
                let angle = self.rng.gen_range(0.0..std::f64::consts::TAU);
                let distance = self.rng.gen_range(15.0..30.0);
                
                let target_x = (cities[i].x as f64 + angle.cos() * distance) as usize;
                let target_y = (cities[i].y as f64 + angle.sin() * distance) as usize;
                
                if target_x < terrain[0].len() && target_y < terrain.len() {
                    // Generate a partial path that might not reach the target
                    let path = self.find_partial_path(terrain, cities[i].x, cities[i].y, target_x, target_y);
                    if path.len() > 5 { // Only add if it's a meaningful path
                        // Detect bridges
                        let mut bridges = Vec::new();
                        for &(x, y) in &path {
                            if river_points.contains(&(x, y)) || 
                               (terrain[y][x].biome == Biome::River && !matches!(terrain[y][x].biome, Biome::Ocean | Biome::DeepOcean | Biome::Lake)) {
                                let bridge = Bridge {
                                    x,
                                    y,
                                    name: self.generate_bridge_name(all_bridges.len()),
                                };
                                bridges.push(bridge.clone());
                                all_bridges.push(bridge);
                            }
                        }
                        
                        roads.push(Road {
                            path,
                            name: format!("Old {} Trail", self.generate_road_name(roads.len())),
                            road_type: "trail".to_string(),
                            bridges,
                        });
                    }
                }
            }
        }
        
        (roads, all_bridges)
    }
    
    fn find_path(&mut self, terrain: &Vec<Vec<TerrainPoint>>, x1: usize, y1: usize, x2: usize, y2: usize) -> Vec<(usize, usize)> {
        // A* pathfinding that avoids water bodies but can cross rivers
        use std::collections::{BinaryHeap, HashMap};
        use std::cmp::Ordering;
        
        #[derive(Copy, Clone, Eq, PartialEq)]
        struct State {
            cost: usize,
            position: (usize, usize),
        }
        
        impl Ord for State {
            fn cmp(&self, other: &Self) -> Ordering {
                other.cost.cmp(&self.cost)
            }
        }
        
        impl PartialOrd for State {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }
        
        let mut dist: HashMap<(usize, usize), usize> = HashMap::new();
        let mut heap = BinaryHeap::new();
        let mut came_from: HashMap<(usize, usize), (usize, usize)> = HashMap::new();
        
        dist.insert((x1, y1), 0);
        heap.push(State { cost: 0, position: (x1, y1) });
        
        while let Some(State { cost, position }) = heap.pop() {
            let (x, y) = position;
            
            if position == (x2, y2) {
                // Reconstruct path
                let mut path = Vec::new();
                let mut current = (x2, y2);
                path.push(current);
                
                while let Some(&prev) = came_from.get(&current) {
                    path.push(prev);
                    current = prev;
                }
                
                path.reverse();
                // Smooth the path to make it more natural
                return self.smooth_path(path, terrain);
            }
            
            if cost > *dist.get(&position).unwrap_or(&usize::MAX) {
                continue;
            }
            
            // Check all 8 neighbors
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 { continue; }
                    
                    let nx = (x as i32 + dx) as usize;
                    let ny = (y as i32 + dy) as usize;
                    
                    if nx >= terrain[0].len() || ny >= terrain.len() {
                        continue;
                    }
                    
                    let next_terrain = &terrain[ny][nx];
                    
                    // Cannot cross oceans or lakes
                    if matches!(next_terrain.biome, Biome::Ocean | Biome::DeepOcean | Biome::Lake | Biome::Shore) {
                        continue;
                    }
                    
                    // Calculate cost - consider elevation changes and terrain type
                    let is_diagonal = dx.abs() + dy.abs() == 2;
                    let mut move_cost = if is_diagonal { 14 } else { 10 };
                    
                    // Add cost for elevation changes (roads prefer flat terrain)
                    let current_elevation = terrain[y][x].elevation;
                    let next_elevation = next_terrain.elevation;
                    let elevation_change = (next_elevation - current_elevation).abs();
                    
                    // Heavy penalty for elevation changes
                    move_cost += (elevation_change * 100.0) as usize;
                    
                    // Additional terrain-based costs
                    if next_terrain.biome == Biome::River {
                        move_cost *= 5; // Rivers are expensive to cross (bridges needed)
                    } else if next_terrain.biome == Biome::Mountains {
                        move_cost *= 8; // Mountains are very hard to cross
                    } else if next_terrain.biome == Biome::SnowPeaks {
                        move_cost *= 10; // Snow peaks are nearly impassable
                    } else if next_terrain.biome == Biome::Hills {
                        move_cost *= 2; // Hills are moderately difficult
                    } else if next_terrain.biome == Biome::Swamp {
                        move_cost *= 3; // Swamps are difficult
                    } else if next_terrain.biome == Biome::Forest {
                        move_cost = (move_cost as f32 * 1.5) as usize; // Forests slow travel
                    }
                    
                    // Add MORE random variation to prevent unnaturally straight lines
                    move_cost += self.rng.gen_range(5..35);
                    
                    // EXTREME penalties for horizontal/vertical movement and right angles
                    if !is_diagonal {
                        // Check if we're creating a right angle or continuing straight
                        if let Some(&prev_pos) = came_from.get(&position) {
                            let prev_dx = x as i32 - prev_pos.0 as i32;
                            let prev_dy = y as i32 - prev_pos.1 as i32;
                            
                            // CRITICAL: Detect right angles (90-degree turns)
                            // Right angle cases:
                            // 1. Was diagonal, now horizontal/vertical
                            // 2. Was horizontal, now vertical (or vice versa)
                            let is_right_angle = 
                                // From diagonal to straight
                                ((prev_dx != 0 && prev_dy != 0) && (dx == 0 || dy == 0)) ||
                                // From horizontal to vertical
                                (prev_dx != 0 && prev_dy == 0 && dx == 0 && dy != 0) ||
                                // From vertical to horizontal  
                                (prev_dx == 0 && prev_dy != 0 && dx != 0 && dy == 0);
                            
                            if is_right_angle {
                                // PROHIBITIVE penalty for creating right angles
                                move_cost += 1000;
                            } else {
                                // Count consecutive straight moves
                                let mut straight_count = 0;
                                let mut check_pos = position;
                                while let Some(&prev) = came_from.get(&check_pos) {
                                    let check_dx = check_pos.0 as i32 - prev.0 as i32;
                                    let check_dy = check_pos.1 as i32 - prev.1 as i32;
                                    // Count if continuing in exact same direction
                                    if check_dx == dx && check_dy == dy {
                                        straight_count += 1;
                                        check_pos = prev;
                                    } else {
                                        break;
                                    }
                                }
                                
                                // Exponentially increasing penalty for straight lines
                                if straight_count > 0 {
                                    move_cost += straight_count * straight_count * 50; // Much higher multiplier
                                }
                                
                                // Base penalty for any horizontal/vertical movement
                                if dx == 0 || dy == 0 {
                                    move_cost += 200 + self.rng.gen_range(50..100);
                                    
                                    // Extra penalty if continuing horizontal/vertical
                                    if (dx == 0 && prev_dx == 0) || (dy == 0 && prev_dy == 0) {
                                        move_cost += 300;
                                    }
                                }
                            }
                        } else if dx == 0 || dy == 0 {
                            // First move being horizontal/vertical gets significant penalty
                            move_cost += 150;
                        }
                    } else {
                        // STRONGLY prefer diagonal movement for natural curves
                        move_cost = (move_cost as f32 * 0.3) as usize; // Much stronger diagonal preference
                        
                        // Bonus for smooth diagonal transitions
                        if let Some(&prev_pos) = came_from.get(&position) {
                            let prev_dx = x as i32 - prev_pos.0 as i32;
                            let prev_dy = y as i32 - prev_pos.1 as i32;
                            
                            // If continuing smoothly from another diagonal
                            if prev_dx != 0 && prev_dy != 0 {
                                // Same diagonal or gentle curve
                                let angle_change = (dx - prev_dx).abs() + (dy - prev_dy).abs();
                                if angle_change <= 1 {
                                    move_cost = (move_cost as f32 * 0.5) as usize; // Big bonus for smooth curves
                                }
                            }
                        }
                    }
                    
                    // Prefer following contours (moving along similar elevation)
                    if elevation_change < 0.05 {
                        move_cost = (move_cost as f32 * 0.85) as usize; // Stronger preference for contour following
                    }
                    
                    // Use EUCLIDEAN distance for more natural, diagonal-friendly paths
                    let dx_goal = (nx as f32 - x2 as f32);
                    let dy_goal = (ny as f32 - y2 as f32);
                    let heuristic = ((dx_goal * dx_goal + dy_goal * dy_goal).sqrt() * 12.0) as usize;
                    let next = State { cost: cost + move_cost + heuristic / 4, position: (nx, ny) };
                    
                    if next.cost < *dist.get(&next.position).unwrap_or(&usize::MAX) {
                        heap.push(next);
                        dist.insert(next.position, next.cost);
                        came_from.insert(next.position, position);
                    }
                }
            }
        }
        
        Vec::new() // No path found
    }
    
    fn smooth_path(&mut self, path: Vec<(usize, usize)>, terrain: &Vec<Vec<TerrainPoint>>) -> Vec<(usize, usize)> {
        if path.len() < 3 {
            return path;
        }
        
        let mut smoothed = Vec::new();
        
        // First pass: Smooth out sharp corners with larger radius curves
        let mut splined = Vec::new();
        
        if path.len() < 3 {
            return path;
        }
        
        splined.push(path[0]);
        
        for i in 1..path.len() - 1 {
            let prev = path[i - 1];
            let curr = path[i];
            let next = path[i + 1];
            
            // Calculate vectors
            let v1x = curr.0 as f32 - prev.0 as f32;
            let v1y = curr.1 as f32 - prev.1 as f32;
            let v2x = next.0 as f32 - curr.0 as f32;
            let v2y = next.1 as f32 - curr.1 as f32;
            
            // Check for ANY direction change that needs smoothing
            // Calculate the dot product to detect angle changes
            let dot_product = v1x * v2x + v1y * v2y;
            let v1_len = (v1x * v1x + v1y * v1y).sqrt();
            let v2_len = (v2x * v2x + v2y * v2y).sqrt();
            
            // Normalize and calculate angle
            let cos_angle = if v1_len > 0.0 && v2_len > 0.0 {
                dot_product / (v1_len * v2_len)
            } else {
                1.0
            };
            
            // Detect right angles and sharp turns
            let is_right_angle = cos_angle.abs() < 0.1; // ~90 degrees
            let is_sharp_turn = cos_angle < 0.5; // More than 60 degrees
            
            // Always smooth any significant direction change
            if is_right_angle || is_sharp_turn || (v1x * v2x + v1y * v2y) <= 0.5 {
                // Calculate curve radius based on the angle
                let dist1 = (v1x * v1x + v1y * v1y).sqrt();
                let dist2 = (v2x * v2x + v2y * v2y).sqrt();
                let curve_radius = dist1.min(dist2) * 0.4; // Use 40% of the shorter segment
                
                // Create control points for a smooth curve
                let t1 = curve_radius / dist1;
                let t2 = curve_radius / dist2;
                
                let p1x = curr.0 as f32 - v1x * t1;
                let p1y = curr.1 as f32 - v1y * t1;
                let p2x = curr.0 as f32 + v2x * t2;
                let p2y = curr.1 as f32 + v2y * t2;
                
                // Generate curve points using cubic Bezier
                for j in 0..=8 {
                    let t = j as f32 / 8.0;
                    let t2 = t * t;
                    let t3 = t2 * t;
                    let mt = 1.0 - t;
                    let mt2 = mt * mt;
                    let mt3 = mt2 * mt;
                    
                    // Cubic Bezier formula
                    let px = mt3 * p1x + 3.0 * mt2 * t * curr.0 as f32 + 3.0 * mt * t2 * curr.0 as f32 + t3 * p2x;
                    let py = mt3 * p1y + 3.0 * mt2 * t * curr.1 as f32 + 3.0 * mt * t2 * curr.1 as f32 + t3 * p2y;
                    
                    let curved_x = px.round().max(0.0) as usize;
                    let curved_y = py.round().max(0.0) as usize;
                    
                    if curved_x < terrain[0].len() && curved_y < terrain.len() {
                        // Check it's not water
                        if !matches!(terrain[curved_y][curved_x].biome,
                                    Biome::Ocean | Biome::DeepOcean | Biome::Lake) {
                            splined.push((curved_x, curved_y));
                        }
                    }
                }
            } else {
                // Keep the original point for straight segments
                splined.push(curr);
            }
        }
        
        splined.push(path[path.len() - 1]);
        
        // Second pass: Add natural wiggles to straight segments
        smoothed.push(splined[0]);
        
        let mut i = 0;
        while i < splined.len() - 1 {
            let current = splined[i];
            let next = splined[i + 1];
            
            let dx = next.0 as f32 - current.0 as f32;
            let dy = next.1 as f32 - current.1 as f32;
            let distance = (dx * dx + dy * dy).sqrt();
            
            // For segments longer than 1 unit, add subtle wiggles
            if distance > 1.5 {
                let num_points = (distance * 0.8) as usize;
                
                // Generate a smooth noise curve for this segment
                let phase = self.rng.gen_range(0.0..std::f32::consts::TAU);
                let frequency = self.rng.gen_range(0.3..0.7);
                let amplitude = self.rng.gen_range(0.5..1.2);
                
                for j in 1..=num_points {
                    let t = j as f32 / (num_points + 1) as f32;
                    
                    // Base position along the line
                    let base_x = current.0 as f32 + dx * t;
                    let base_y = current.1 as f32 + dy * t;
                    
                    // Calculate perpendicular direction
                    let perp_x = -dy / distance;
                    let perp_y = dx / distance;
                    
                    // Add smooth wiggle using multiple sine waves for natural look
                    let wiggle1 = (t * std::f32::consts::PI * frequency + phase).sin() * amplitude;
                    let wiggle2 = (t * std::f32::consts::PI * frequency * 2.3 + phase * 0.7).sin() * amplitude * 0.3;
                    let total_wiggle = wiggle1 + wiggle2;
                    
                    // Apply wiggle perpendicular to the road direction
                    let wiggle_x = base_x + perp_x * total_wiggle;
                    let wiggle_y = base_y + perp_y * total_wiggle;
                    
                    // Add very small random variation for natural imperfection
                    let final_x = (wiggle_x + self.rng.gen_range(-0.1..0.1)).round() as usize;
                    let final_y = (wiggle_y + self.rng.gen_range(-0.1..0.1)).round() as usize;
                    
                    // Ensure the point is valid and preferably not in water
                    if final_x < terrain[0].len() && final_y < terrain.len() {
                        if !matches!(terrain[final_y][final_x].biome,
                                    Biome::Ocean | Biome::DeepOcean | Biome::Lake) {
                            smoothed.push((final_x, final_y));
                        } else {
                            // Fall back to straight line if curve goes into water
                            smoothed.push((base_x.round() as usize, base_y.round() as usize));
                        }
                    }
                }
            }
            
            smoothed.push(next);
            i += 1;
        }
        
        // Remove any duplicate consecutive points
        let mut deduped = Vec::new();
        let mut last = (usize::MAX, usize::MAX);
        for point in smoothed {
            if point != last {
                deduped.push(point);
                last = point;
            }
        }
        
        deduped
    }
    
    fn can_connect_smoothly(&self, from: (usize, usize), to: (usize, usize), terrain: &Vec<Vec<TerrainPoint>>) -> bool {
        // Check if we can connect two points without crossing water
        let dx = to.0 as i32 - from.0 as i32;
        let dy = to.1 as i32 - from.1 as i32;
        let steps = dx.abs().max(dy.abs()) as usize;
        
        if steps == 0 {
            return true;
        }
        
        for i in 1..steps {
            let t = i as f32 / steps as f32;
            let x = (from.0 as f32 + dx as f32 * t) as usize;
            let y = (from.1 as f32 + dy as f32 * t) as usize;
            
            if x >= terrain[0].len() || y >= terrain.len() {
                return false;
            }
            
            // Can't cross water
            if matches!(terrain[y][x].biome, 
                Biome::Ocean | Biome::DeepOcean | Biome::Lake | Biome::Shore) {
                return false;
            }
        }
        
        true
    }
    
    fn find_partial_path(&mut self, terrain: &Vec<Vec<TerrainPoint>>, x1: usize, y1: usize, x2: usize, y2: usize) -> Vec<(usize, usize)> {
        // Similar to find_path but stops early if terrain becomes too difficult
        use std::collections::{BinaryHeap, HashMap};
        use std::cmp::Ordering;
        
        #[derive(Copy, Clone, Eq, PartialEq)]
        struct State {
            cost: usize,
            position: (usize, usize),
        }
        
        impl Ord for State {
            fn cmp(&self, other: &Self) -> Ordering {
                other.cost.cmp(&self.cost)
            }
        }
        
        impl PartialOrd for State {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }
        
        let mut dist: HashMap<(usize, usize), usize> = HashMap::new();
        let mut heap = BinaryHeap::new();
        let mut came_from: HashMap<(usize, usize), (usize, usize)> = HashMap::new();
        let max_steps = 50; // Limit path length
        let mut steps = 0;
        
        dist.insert((x1, y1), 0);
        heap.push(State { cost: 0, position: (x1, y1) });
        
        while let Some(State { cost, position }) = heap.pop() {
            let (x, y) = position;
            steps += 1;
            
            // Stop if we've gone far enough or reached difficult terrain
            if steps > max_steps || 
               matches!(terrain[y][x].biome, Biome::Mountains | Biome::SnowPeaks | Biome::Ocean | Biome::DeepOcean) {
                // Return partial path
                let mut path = Vec::new();
                let mut current = position;
                path.push(current);
                
                while let Some(&prev) = came_from.get(&current) {
                    path.push(prev);
                    current = prev;
                }
                
                path.reverse();
                return self.smooth_path(path, terrain);
            }
            
            if position == (x2, y2) {
                // Found complete path
                let mut path = Vec::new();
                let mut current = (x2, y2);
                path.push(current);
                
                while let Some(&prev) = came_from.get(&current) {
                    path.push(prev);
                    current = prev;
                }
                
                path.reverse();
                return self.smooth_path(path, terrain);
            }
            
            if cost > *dist.get(&position).unwrap_or(&usize::MAX) {
                continue;
            }
            
            // Check neighbors
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 { continue; }
                    
                    let nx = (x as i32 + dx) as usize;
                    let ny = (y as i32 + dy) as usize;
                    
                    if nx >= terrain[0].len() || ny >= terrain.len() {
                        continue;
                    }
                    
                    let next_terrain = &terrain[ny][nx];
                    
                    // Cannot cross water
                    if matches!(next_terrain.biome, Biome::Ocean | Biome::DeepOcean | Biome::Lake | Biome::Shore) {
                        continue;
                    }
                    
                    // Calculate cost
                    let mut move_cost = if dx.abs() + dy.abs() == 2 { 14 } else { 10 };
                    let elevation_change = (next_terrain.elevation - terrain[y][x].elevation).abs();
                    move_cost += (elevation_change * 50.0) as usize;
                    
                    let next = State { cost: cost + move_cost, position: (nx, ny) };
                    
                    if next.cost < *dist.get(&next.position).unwrap_or(&usize::MAX) {
                        heap.push(next);
                        dist.insert(next.position, next.cost);
                        came_from.insert(next.position, position);
                    }
                }
            }
        }
        
        Vec::new()
    }
    
    fn find_nearest_city(&self, cities: &Vec<City>, current: usize) -> Option<usize> {
        let mut min_dist = f64::MAX;
        let mut nearest = None;
        
        for i in 0..cities.len() {
            if i != current {
                let dx = cities[current].x as f64 - cities[i].x as f64;
                let dy = cities[current].y as f64 - cities[i].y as f64;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < min_dist {
                    min_dist = dist;
                    nearest = Some(i);
                }
            }
        }
        
        nearest
    }
    
    fn generate_labels(&mut self, terrain: &Vec<Vec<TerrainPoint>>, rivers: &Vec<Vec<(usize, usize)>>, _cities: &Vec<City>) -> Vec<PlaceLabel> {
        let mut labels = Vec::new();
        let mut placed_labels: Vec<(f32, f32)> = Vec::new();
        
        // Helper to check if a label position is too close to existing labels
        // Scale minimum distance based on map size
        let map_scale = (terrain[0].len() as f32 / 160.0).max(terrain.len() as f32 / 120.0);
        let min_distance = 80.0 * map_scale; // Minimum distance between labels
        let is_too_close = |x: f32, y: f32, placed: &Vec<(f32, f32)>| -> bool {
            for &(px, py) in placed {
                let dist = ((x - px).powi(2) + (y - py).powi(2)).sqrt();
                if dist < min_distance {
                    return true;
                }
            }
            false
        };
        
        // Ocean names - only label the largest oceans
        let ocean_regions = self.find_regions(terrain, |b| matches!(b, Biome::Ocean | Biome::DeepOcean));
        let mut ocean_regions_sorted: Vec<_> = ocean_regions.iter().enumerate().collect();
        ocean_regions_sorted.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
        
        for (i, (idx, region)) in ocean_regions_sorted.iter().take(3).enumerate() {
            if region.len() > 200 {
                let (cx, cy) = self.region_center(region);
                let fx = cx as f32;
                let fy = cy as f32;
                if !is_too_close(fx, fy, &placed_labels) {
                    labels.push(PlaceLabel {
                        x: fx,
                        y: fy,
                        name: self.generate_ocean_name(*idx),
                        feature_type: "ocean".to_string(),
                    });
                    placed_labels.push((fx, fy));
                }
            }
        }
        
        // Mountain range names - only major ranges
        let mountain_regions = self.find_regions(terrain, |b| matches!(b, Biome::Mountains | Biome::SnowPeaks));
        let mut mountain_regions_sorted: Vec<_> = mountain_regions.iter().enumerate().collect();
        mountain_regions_sorted.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
        
        for (i, (idx, region)) in mountain_regions_sorted.iter().take(4).enumerate() {
            if region.len() > 40 {
                let (cx, cy) = self.region_center(region);
                let fx = cx as f32;
                let fy = cy as f32;
                if !is_too_close(fx, fy, &placed_labels) {
                    labels.push(PlaceLabel {
                        x: fx,
                        y: fy,
                        name: self.generate_mountain_name(*idx),
                        feature_type: "mountains".to_string(),
                    });
                    placed_labels.push((fx, fy));
                }
            }
        }
        
        // Forest names - only large forests
        let forest_regions = self.find_regions(terrain, |b| matches!(b, Biome::Forest));
        let mut forest_regions_sorted: Vec<_> = forest_regions.iter().enumerate().collect();
        forest_regions_sorted.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
        
        for (i, (idx, region)) in forest_regions_sorted.iter().take(3).enumerate() {
            if region.len() > 100 {
                let (cx, cy) = self.region_center(region);
                let fx = cx as f32;
                let fy = cy as f32;
                if !is_too_close(fx, fy, &placed_labels) {
                    labels.push(PlaceLabel {
                        x: fx,
                        y: fy,
                        name: self.generate_forest_name(*idx),
                        feature_type: "forest".to_string(),
                    });
                    placed_labels.push((fx, fy));
                }
            }
        }
        
        // Swamp names - only major swamps
        let swamp_regions = self.find_regions(terrain, |b| matches!(b, Biome::Swamp));
        let mut swamp_regions_sorted: Vec<_> = swamp_regions.iter().enumerate().collect();
        swamp_regions_sorted.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
        
        for (i, (idx, region)) in swamp_regions_sorted.iter().take(2).enumerate() {
            if region.len() > 60 {
                let (cx, cy) = self.region_center(region);
                let fx = cx as f32;
                let fy = cy as f32;
                if !is_too_close(fx, fy, &placed_labels) {
                    labels.push(PlaceLabel {
                        x: fx,
                        y: fy,
                        name: self.generate_swamp_name(*idx),
                        feature_type: "swamp".to_string(),
                    });
                    placed_labels.push((fx, fy));
                }
            }
        }
        
        // River names - only major rivers, well-spaced
        let mut river_labels_added = 0;
        for (i, river) in rivers.iter().enumerate() {
            if river.len() > 30 && river_labels_added < 3 {
                // Place label at a good position along the river
                let positions = [river.len() / 3, river.len() / 2, river.len() * 2 / 3];
                for pos in positions {
                    if pos < river.len() {
                        let fx = river[pos].0 as f32;
                        let fy = river[pos].1 as f32;
                        if !is_too_close(fx, fy, &placed_labels) {
                            labels.push(PlaceLabel {
                                x: fx,
                                y: fy,
                                name: self.generate_river_name(i),
                                feature_type: "river".to_string(),
                            });
                            placed_labels.push((fx, fy));
                            river_labels_added += 1;
                            break;
                        }
                    }
                }
            }
        }
        
        labels
    }
    
    fn find_regions(&self, terrain: &Vec<Vec<TerrainPoint>>, predicate: fn(&Biome) -> bool) -> Vec<Vec<(usize, usize)>> {
        let mut regions = Vec::new();
        let mut visited = vec![vec![false; terrain[0].len()]; terrain.len()];
        
        for y in 0..terrain.len() {
            for x in 0..terrain[0].len() {
                if !visited[y][x] && predicate(&terrain[y][x].biome) {
                    let mut region = Vec::new();
                    let mut stack = vec![(x, y)];
                    
                    while let Some((cx, cy)) = stack.pop() {
                        if visited[cy][cx] {
                            continue;
                        }
                        
                        visited[cy][cx] = true;
                        region.push((cx, cy));
                        
                        for dy in -1i32..=1 {
                            for dx in -1i32..=1 {
                                if dx == 0 && dy == 0 {
                                    continue;
                                }
                                
                                let nx = cx as i32 + dx;
                                let ny = cy as i32 + dy;
                                
                                if nx >= 0 && nx < terrain[0].len() as i32 && 
                                   ny >= 0 && ny < terrain.len() as i32 {
                                    let nx = nx as usize;
                                    let ny = ny as usize;
                                    
                                    if !visited[ny][nx] && predicate(&terrain[ny][nx].biome) {
                                        stack.push((nx, ny));
                                    }
                                }
                            }
                        }
                    }
                    
                    if region.len() > 10 {
                        regions.push(region);
                    }
                }
            }
        }
        
        regions
    }
    
    fn region_center(&self, region: &Vec<(usize, usize)>) -> (usize, usize) {
        // For water regions, find the point that's farthest from any land
        // This ensures ocean labels are placed in open water
        let mut best_pos = (0, 0);
        let mut max_dist_to_edge = 0;
        
        // Sample some points in the region to find the best label position
        let sample_rate = (region.len() / 100).max(1);
        for (i, &(x, y)) in region.iter().enumerate() {
            if i % sample_rate != 0 { continue; } // Sample to reduce computation
            
            // Find minimum distance to edge of region (approximation of distance to land)
            let mut min_dist = usize::MAX;
            for &(ox, oy) in region.iter().step_by(sample_rate * 5) {
                let dx = if x > ox { x - ox } else { ox - x };
                let dy = if y > oy { y - oy } else { oy - y };
                let dist = dx + dy; // Manhattan distance for speed
                
                // Check if this point is at the edge of the region (next to non-region)
                let is_edge = !region.contains(&(ox + 1, oy)) || 
                             !region.contains(&(ox, oy + 1)) ||
                             (ox > 0 && !region.contains(&(ox - 1, oy))) ||
                             (oy > 0 && !region.contains(&(ox, oy - 1)));
                
                if is_edge && dist < min_dist {
                    min_dist = dist;
                }
            }
            
            if min_dist > max_dist_to_edge {
                max_dist_to_edge = min_dist;
                best_pos = (x, y);
            }
        }
        
        // If we didn't find a good position, fall back to simple center
        if max_dist_to_edge == 0 {
            let sum_x: usize = region.iter().map(|(x, _)| x).sum();
            let sum_y: usize = region.iter().map(|(_, y)| y).sum();
            (sum_x / region.len(), sum_y / region.len())
        } else {
            best_pos
        }
    }
    
    fn generate_ocean_name(&mut self, _index: usize) -> String {
        let prefixes = ["Azure", "Cerulean", "Sapphire", "Mystic", "Crystal", "Eternal", "Whispering"];
        let suffixes = ["Sea", "Ocean", "Deep", "Abyss", "Waters", "Expanse", "Bay"];
        let prefix = prefixes[self.rng.gen_range(0..prefixes.len())];
        let suffix = suffixes[self.rng.gen_range(0..suffixes.len())];
        format!("{} {}", prefix, suffix)
    }
    
    fn generate_mountain_name(&mut self, index: usize) -> String {
        let prefixes = ["Mount", "Mt.", "Peak"];
        let first_parts = ["Storm", "Iron", "Snow", "Thunder", "Eagle", "Wolf", "Dragon", "Crystal", 
                          "Shadow", "Silver", "Golden", "Frost", "Wind", "Cloud", "Stone", "Red"];
        let second_parts = ["horn", "crest", "spire", "ridge", "tooth", "peak", "crown", "fang", 
                           "head", "point", "top", "summit", "needle", "wall"];
        let suffixes = ["Mountains", "Range", "Peaks", "Heights", "Alps", "Highlands"];
        
        // Ensure variety by using index to influence selection
        let prefix_idx = (index + self.rng.gen_range(0..3)) % prefixes.len();
        let first_idx = (index * 7 + self.rng.gen_range(0..4)) % first_parts.len();
        let second_idx = (index * 5 + self.rng.gen_range(0..3)) % second_parts.len();
        
        if self.rng.gen_bool(0.4) {
            // Sometimes just use a suffix for the range
            let suffix = suffixes[self.rng.gen_range(0..suffixes.len())];
            format!("The {} {}", 
                   format!("{}{}", first_parts[first_idx], second_parts[second_idx]),
                   suffix)
        } else {
            format!("{} {}{}", 
                   prefixes[prefix_idx], 
                   first_parts[first_idx], 
                   second_parts[second_idx])
        }
    }
    
    fn generate_forest_name(&mut self, _index: usize) -> String {
        let adjectives = ["Whispering", "Ancient", "Enchanted", "Dark", "Silver", "Golden", "Misty"];
        let nouns = ["Woods", "Forest", "Grove", "Thicket", "Woodland", "Glade", "Copse"];
        let adj = adjectives[self.rng.gen_range(0..adjectives.len())];
        let noun = nouns[self.rng.gen_range(0..nouns.len())];
        format!("{} {}", adj, noun)
    }
    
    fn generate_swamp_name(&mut self, _index: usize) -> String {
        let adjectives = ["Murky", "Fetid", "Misty", "Black", "Forgotten", "Cursed", "Silent"];
        let nouns = ["Marsh", "Swamp", "Bog", "Fen", "Mire", "Wetlands", "Quagmire"];
        let adj = adjectives[self.rng.gen_range(0..adjectives.len())];
        let noun = nouns[self.rng.gen_range(0..nouns.len())];
        format!("{} {}", adj, noun)
    }
    
    fn generate_city_name(&mut self, index: usize) -> String {
        let prefixes = ["New", "Port", "Fort", "Saint", "North", "South", "East", "West", "Old", ""];
        let first_parts = ["Oak", "River", "Lake", "Hill", "Green", "White", "Black", "Gold", "Silver",
                          "Spring", "Summer", "Winter", "Mill", "Fair", "Clear", "Bright"];
        let second_parts = ["haven", "bridge", "vale", "crest", "shore", "field", "gate", "wells",
                           "cross", "wood", "meadow", "ridge", "view", "hill", "brook"];
        let city_suffixes = ["ton", "ville", "burg", "shire", "ford", "mouth", "stead", "ham", "thorpe"];
        let city_types = [" City", " Town", "", "", ""];  // Sometimes add City/Town
        
        // Use index to ensure variety
        let prefix_chance = self.rng.gen_bool(0.4);
        let first_idx = (index * 3 + self.rng.gen_range(0..4)) % first_parts.len();
        let second_idx = (index * 5 + self.rng.gen_range(0..3)) % second_parts.len();
        
        let base_name = if self.rng.gen_bool(0.6) {
            // Compound name with suffix
            let suffix = city_suffixes[(index * 7 + self.rng.gen_range(0..2)) % city_suffixes.len()];
            format!("{}{}{}", first_parts[first_idx], second_parts[second_idx], suffix)
        } else {
            // Two-part name
            format!("{}{}", first_parts[first_idx].to_string(), second_parts[second_idx])
        };
        
        let with_prefix = if prefix_chance {
            let prefix = prefixes[self.rng.gen_range(0..prefixes.len())];
            if prefix.is_empty() {
                base_name
            } else {
                format!("{} {}", prefix, base_name)
            }
        } else {
            base_name
        };
        
        // Add City/Town suffix for clarity
        let city_type = city_types[self.rng.gen_range(0..city_types.len())];
        format!("{}{}", with_prefix, city_type)
    }
    
    fn generate_road_name(&mut self, index: usize) -> String {
        let descriptors = ["King's", "Queen's", "Merchant's", "Old", "Ancient", "Royal",
                          "Imperial", "Trade", "Coastal", "Mountain", "Forest", "Valley",
                          "Pioneer", "Settler's", "Hunter's", "Pilgrim's"];
        // Use index to ensure variety
        let desc_idx = (index * 3 + self.rng.gen_range(0..4)) % descriptors.len();
        descriptors[desc_idx].to_string()
    }
    
    fn generate_river_name(&mut self, _index: usize) -> String {
        let prefixes = ["River", "The"];
        let names = ["Silverflow", "Clearwater", "Rushing", "Serpent", "Crystal", "Moonwater", "Swift"];
        let prefix = prefixes[self.rng.gen_range(0..prefixes.len())];
        let name = names[self.rng.gen_range(0..names.len())];
        
        if prefix == "The" {
            format!("{} {} River", prefix, name)
        } else {
            format!("{} {}", name, prefix)
        }
    }
    
    fn generate_bridge_name(&mut self, index: usize) -> String {
        let prefixes = ["Old", "New", "Great", "High", "Stone", "Iron", "Wooden", "Ancient"];
        let middles = ["River", "Creek", "Valley", "Canyon", "Gorge", "Falls", "Rapids", "Mill"];
        
        // Always make it clear it's a bridge
        let prefix_idx = (index * 5 + self.rng.gen_range(0..3)) % prefixes.len();
        let middle_idx = (index * 3 + self.rng.gen_range(0..2)) % middles.len();
        
        format!("{} {} Bridge", prefixes[prefix_idx], middles[middle_idx])
    }
}

impl Biome {
    pub fn color(&self) -> [u8; 4] {
        match self {
            Biome::DeepOcean => [0, 20, 80, 255],      // Very dark blue (no grey)
            Biome::Ocean => [5, 40, 120, 255],         // Dark ocean blue (more blue)
            Biome::Shore => [20, 70, 160, 255],        // Bright blue shallow water (vivid blue)
            Biome::Beach => [220, 200, 160, 255],      // Light brown/tan sand
            Biome::Plains => [120, 180, 90, 255],      // Light green grassland
            Biome::Forest => [50, 120, 50, 255],       // Forest green
            Biome::Hills => [140, 160, 100, 255],      // Brown-green
            Biome::Mountains => [140, 130, 120, 255],  // Gray-brown
            Biome::SnowPeaks => [245, 245, 250, 255],  // Snow white
            Biome::River => [20, 60, 120, 255],        // Dark river blue
            Biome::Lake => [15, 55, 100, 255],         // Dark lake blue
            Biome::Swamp => [60, 80, 60, 255],         // Swamp green-brown
            Biome::Desert => [230, 210, 170, 255],     // Desert sand (lighter than beach)
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
            let r = (140.0) as u8;
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
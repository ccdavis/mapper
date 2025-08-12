use crate::terrain_generator::{TerrainMap, Biome};
use image::{ImageBuffer, Rgb, RgbImage};

pub struct TerrainRenderer;

impl TerrainRenderer {
    /// Renders a terrain map to RGB pixel data
    pub fn render_to_pixels(
        map: &TerrainMap,
        width: usize,
        height: usize,
        scale: usize,
    ) -> Vec<u8> {
        let img_width = width * scale;
        let img_height = height * scale;
        let mut pixels = vec![0u8; img_width * img_height * 4];
        
        // Helper function to get terrain color with smooth coastlines
        let get_terrain_color = |x: f32, y: f32| -> [f32; 3] {
            let x0 = x.floor() as usize;
            let y0 = y.floor() as usize;
            let x1 = (x0 + 1).min(width - 1);
            let y1 = (y0 + 1).min(height - 1);
            
            let fx = x - x0 as f32;
            let fy = y - y0 as f32;
            
            // Get the four corner points with elevation data
            let get_point_data = |px: usize, py: usize| -> ([f32; 3], bool, f32) {
                if px < width && py < height {
                    let terrain_point = &map.terrain[py][px];
                    let base_color = Biome::elevation_color(terrain_point.elevation);
                    let biome_color = terrain_point.biome.color();
                    let blend_factor = 0.7;
                    let color = [
                        base_color[0] as f32 * (1.0 - blend_factor) + biome_color[0] as f32 * blend_factor,
                        base_color[1] as f32 * (1.0 - blend_factor) + biome_color[1] as f32 * blend_factor,
                        base_color[2] as f32 * (1.0 - blend_factor) + biome_color[2] as f32 * blend_factor,
                    ];
                    let is_water = matches!(terrain_point.biome, 
                        Biome::Ocean | Biome::DeepOcean | Biome::Shore | Biome::Lake);
                    (color, is_water, terrain_point.elevation as f32)
                } else {
                    ([0.0, 0.0, 0.0], false, 0.0)
                }
            };
            
            let (c00, water00, elev00) = get_point_data(x0, y0);
            let (c10, water10, elev10) = get_point_data(x1, y0);
            let (c01, water01, elev01) = get_point_data(x0, y1);
            let (c11, water11, elev11) = get_point_data(x1, y1);
            
            // Check if this is a water-land boundary
            let water_count = [water00, water10, water01, water11].iter().filter(|&&w| w).count();
            
            // If all same type, use smooth interpolation
            if water_count == 0 || water_count == 4 {
                // Bilinear interpolation for smooth transitions
                let mut result = [0.0; 3];
                for i in 0..3 {
                    let c0 = c00[i] * (1.0 - fx) + c10[i] * fx;
                    let c1 = c01[i] * (1.0 - fx) + c11[i] * fx;
                    result[i] = c0 * (1.0 - fy) + c1 * fy;
                }
                result
            } else {
                // Marching squares approach for smooth boundaries
                // Calculate isolevel based on elevation (sea level = 0.5)
                let iso_level = 0.5;
                
                // Get corner values (1 for land, 0 for water)
                let v00 = if water00 { 0.0 } else { 1.0 };
                let v10 = if water10 { 0.0 } else { 1.0 };
                let v01 = if water01 { 0.0 } else { 1.0 };
                let v11 = if water11 { 0.0 } else { 1.0 };
                
                // Bilinear interpolation of the field value
                let v0 = v00 * (1.0 - fx) + v10 * fx;
                let v1 = v01 * (1.0 - fx) + v11 * fx;
                let v = v0 * (1.0 - fy) + v1 * fy;
                
                // Add subtle noise for natural coastlines with smoother curves
                let noise_scale = 5.0;
                let noise_x = x * noise_scale;
                let noise_y = y * noise_scale;
                // Multi-octave noise for more natural appearance
                let noise1 = ((noise_x * 0.7).sin() * 12.9898 + (noise_y * 0.7).cos() * 78.233).sin().abs() * 0.5;
                let noise2 = ((noise_x * 1.4).cos() * 23.456 + (noise_y * 1.4).sin() * 45.678).cos().abs() * 0.25;
                let noise = (noise1 + noise2) * 0.15;
                
                // Smooth the transition with a sigmoid-like curve
                let smooth_v = if v < 0.3 {
                    v * v * 1.111
                } else if v > 0.7 {
                    1.0 - (1.0 - v) * (1.0 - v) * 1.111
                } else {
                    v
                };
                
                let adjusted_v = smooth_v + noise;
                
                // Determine which color to use based on the interpolated value
                if adjusted_v > iso_level {
                    // Land side - interpolate land colors
                    if !water00 && !water10 && !water01 && !water11 {
                        // All land - normal interpolation
                        let mut result = [0.0; 3];
                        for i in 0..3 {
                            let c0 = c00[i] * (1.0 - fx) + c10[i] * fx;
                            let c1 = c01[i] * (1.0 - fx) + c11[i] * fx;
                            result[i] = c0 * (1.0 - fy) + c1 * fy;
                        }
                        result
                    } else {
                        // Mix of land and water - use nearest land color
                        if !water00 { c00 }
                        else if !water10 { c10 }
                        else if !water01 { c01 }
                        else { c11 }
                    }
                } else {
                    // Water side - interpolate water colors
                    if water00 && water10 && water01 && water11 {
                        // All water - normal interpolation
                        let mut result = [0.0; 3];
                        for i in 0..3 {
                            let c0 = c00[i] * (1.0 - fx) + c10[i] * fx;
                            let c1 = c01[i] * (1.0 - fx) + c11[i] * fx;
                            result[i] = c0 * (1.0 - fy) + c1 * fy;
                        }
                        result
                    } else {
                        // Mix of land and water - use nearest water color
                        if water00 { c00 }
                        else if water10 { c10 }
                        else if water01 { c01 }
                        else { c11 }
                    }
                }
            }
        };
        
        // Render each pixel with smooth interpolation
        for py in 0..img_height {
            for px in 0..img_width {
                // Calculate position in terrain space with sub-pixel precision
                let tx = px as f32 / scale as f32;
                let ty = py as f32 / scale as f32;
                
                // Get interpolated color at this position
                let mut color = get_terrain_color(tx, ty);
                
                // Apply some noise for texture at boundaries
                let terrain_x = tx.floor() as usize;
                let terrain_y = ty.floor() as usize;
                
                if terrain_x < width - 1 && terrain_y < height - 1 {
                    // Get smooth interpolated elevation at this exact position
                    let fx = tx - terrain_x as f32;
                    let fy = ty - terrain_y as f32;
                    
                    // Get the four corner elevations
                    let elev00 = map.terrain[terrain_y][terrain_x].elevation;
                    let elev10 = map.terrain[terrain_y][terrain_x + 1].elevation;
                    let elev01 = map.terrain[terrain_y + 1][terrain_x].elevation;
                    let elev11 = map.terrain[terrain_y + 1][terrain_x + 1].elevation;
                    
                    // Bilinear interpolation of elevation
                    let elev0 = elev00 * (1.0 - fx as f64) + elev10 * fx as f64;
                    let elev1 = elev01 * (1.0 - fx as f64) + elev11 * fx as f64;
                    let elev_center = elev0 * (1.0 - fy as f64) + elev1 * fy as f64;
                    
                    // Calculate smoothly interpolated elevations at neighboring positions
                    let sample_dist = 0.1; // Sample distance for gradient calculation
                    
                    // Left position
                    let tx_left = (tx - sample_dist).max(0.0);
                    let x_left = tx_left.floor() as usize;
                    let fx_left = tx_left - x_left as f32;
                    let elev_left = if x_left < width - 1 {
                        let e00 = map.terrain[terrain_y.min(height-1)][x_left].elevation;
                        let e10 = map.terrain[terrain_y.min(height-1)][x_left + 1].elevation;
                        let e01 = map.terrain[(terrain_y + 1).min(height-1)][x_left].elevation;
                        let e11 = map.terrain[(terrain_y + 1).min(height-1)][x_left + 1].elevation;
                        let e0 = e00 * (1.0 - fx_left as f64) + e10 * fx_left as f64;
                        let e1 = e01 * (1.0 - fx_left as f64) + e11 * fx_left as f64;
                        e0 * (1.0 - fy as f64) + e1 * fy as f64
                    } else { elev_center };
                    
                    // Right position
                    let tx_right = (tx + sample_dist).min((width - 1) as f32);
                    let x_right = tx_right.floor() as usize;
                    let fx_right = tx_right - x_right as f32;
                    let elev_right = if x_right < width - 1 {
                        let e00 = map.terrain[terrain_y.min(height-1)][x_right].elevation;
                        let e10 = map.terrain[terrain_y.min(height-1)][(x_right + 1).min(width-1)].elevation;
                        let e01 = map.terrain[(terrain_y + 1).min(height-1)][x_right].elevation;
                        let e11 = map.terrain[(terrain_y + 1).min(height-1)][(x_right + 1).min(width-1)].elevation;
                        let e0 = e00 * (1.0 - fx_right as f64) + e10 * fx_right as f64;
                        let e1 = e01 * (1.0 - fx_right as f64) + e11 * fx_right as f64;
                        e0 * (1.0 - fy as f64) + e1 * fy as f64
                    } else { elev_center };
                    
                    // Up position
                    let ty_up = (ty - sample_dist).max(0.0);
                    let y_up = ty_up.floor() as usize;
                    let fy_up = ty_up - y_up as f32;
                    let elev_up = if y_up < height - 1 {
                        let e00 = map.terrain[y_up][terrain_x.min(width-1)].elevation;
                        let e10 = map.terrain[y_up][(terrain_x + 1).min(width-1)].elevation;
                        let e01 = map.terrain[y_up + 1][terrain_x.min(width-1)].elevation;
                        let e11 = map.terrain[y_up + 1][(terrain_x + 1).min(width-1)].elevation;
                        let e0 = e00 * (1.0 - fx as f64) + e10 * fx as f64;
                        let e1 = e01 * (1.0 - fx as f64) + e11 * fx as f64;
                        e0 * (1.0 - fy_up as f64) + e1 * fy_up as f64
                    } else { elev_center };
                    
                    // Down position
                    let ty_down = (ty + sample_dist).min((height - 1) as f32);
                    let y_down = ty_down.floor() as usize;
                    let fy_down = ty_down - y_down as f32;
                    let elev_down = if y_down < height - 1 {
                        let e00 = map.terrain[y_down][terrain_x.min(width-1)].elevation;
                        let e10 = map.terrain[y_down][(terrain_x + 1).min(width-1)].elevation;
                        let e01 = map.terrain[(y_down + 1).min(height-1)][terrain_x.min(width-1)].elevation;
                        let e11 = map.terrain[(y_down + 1).min(height-1)][(terrain_x + 1).min(width-1)].elevation;
                        let e0 = e00 * (1.0 - fx as f64) + e10 * fx as f64;
                        let e1 = e01 * (1.0 - fx as f64) + e11 * fx as f64;
                        e0 * (1.0 - fy_down as f64) + e1 * fy_down as f64
                    } else { elev_center };
                    
                    let current_terrain = &map.terrain[terrain_y][terrain_x];
                    
                    // Add 3D relief shading based on elevation, not biome type
                    // This prevents visible tile boundaries
                    let is_water = elev_center < 0.5; // Use interpolated elevation to determine water
                    
                    if !is_water {
                        
                        // Calculate gradients using smoothly interpolated elevations
                        // Scale intensity based on actual elevation, not biome type
                        // This creates smooth transitions without tile boundaries
                        let elevation_factor = (elev_center - 0.5).max(0.0).min(1.0) * 2.0; // 0 to 1 for land
                        
                        // Smooth gradient scale based on elevation
                        let gradient_scale = if elev_center > 0.85 {
                            25.0 + elevation_factor * 5.0  // Mountains: 25-30
                        } else if elev_center > 0.7 {
                            15.0 + elevation_factor * 10.0  // Hills: 15-25
                        } else if elev_center > 0.6 {
                            8.0 + elevation_factor * 7.0   // Forests: 8-15
                        } else {
                            3.0 + elevation_factor * 5.0   // Plains: 3-8 (much smoother)
                        };
                        
                        // Use the smoothly interpolated elevations for gradient calculation
                        // Divide by sample_dist*2 to get the actual gradient
                        let dx = (elev_right - elev_left) * gradient_scale / (sample_dist as f64 * 2.0);
                        let dy = (elev_down - elev_up) * gradient_scale / (sample_dist as f64 * 2.0);
                        
                        // Light direction (from northwest: -1, -1, 1)
                        let light_x = -0.7071;
                        let light_y = -0.7071;
                        let light_z = 0.5;
                        
                        // Calculate normal vector
                        let normal_x = -dx;
                        let normal_y = -dy;
                        let normal_z = 1.0;
                        let normal_len = (normal_x * normal_x + normal_y * normal_y + normal_z * normal_z).sqrt();
                        
                        // Normalize
                        let nx = normal_x / normal_len;
                        let ny = normal_y / normal_len;
                        let nz = normal_z / normal_len;
                        
                        // Calculate lighting (dot product)
                        let lighting = (nx * light_x + ny * light_y + nz * light_z).max(0.0);
                        
                        // Add high-frequency detail for wrinkled appearance
                        // Scale based on elevation for smooth transitions
                        let wrinkle_intensity = if elev_center > 0.85 {
                            0.8 + elevation_factor * 0.2  // Mountains: 0.8-1.0
                        } else if elev_center > 0.7 {
                            0.5 + elevation_factor * 0.3  // Hills: 0.5-0.8
                        } else if elev_center > 0.6 {
                            0.2 + elevation_factor * 0.2  // Forests: 0.2-0.4
                        } else {
                            0.05 + elevation_factor * 0.1  // Plains: 0.05-0.15 (very smooth)
                        };
                        
                        let detail_x = tx * 50.0;
                        let detail_y = ty * 50.0;
                        let wrinkle1 = ((detail_x * 0.7).sin() * (detail_y * 0.7).cos()).abs() * 0.3 * wrinkle_intensity as f32;
                        let wrinkle2 = ((detail_x * 1.3).cos() * (detail_y * 1.3).sin()).abs() * 0.2 * wrinkle_intensity as f32;
                        let wrinkle3 = ((detail_x * 2.1).sin() * (detail_y * 2.1).sin()).abs() * 0.1 * wrinkle_intensity as f32;
                        let wrinkle_detail = wrinkle1 + wrinkle2 + wrinkle3;
                        
                        // Combine base lighting with wrinkle detail
                        let combined_lighting = (lighting * 0.7 + wrinkle_detail as f64 * 0.3).min(1.0);
                        
                        // Apply shading with moderated contrast based on elevation
                        // Less contrast in plains for better visibility
                        let contrast_factor = 0.3 + elevation_factor as f32 * 0.4; // 0.3 to 0.7
                        
                        let shade_factor = if combined_lighting > 0.6 {
                            // Lit areas - make brighter
                            1.0 + (combined_lighting - 0.6) as f32 * contrast_factor
                        } else {
                            // Shadow areas - make darker but not too dark
                            0.7 + combined_lighting as f32 * 0.3
                        };
                        
                        // Apply the shading to the color
                        color[0] = (color[0] * shade_factor).min(255.0);
                        color[1] = (color[1] * shade_factor).min(255.0);
                        color[2] = (color[2] * shade_factor).min(255.0);
                        
                        // Add subtle color variation based on slope
                        if dx.abs() > 0.1 || dy.abs() > 0.1 {
                            // Steeper slopes get slightly different color
                            let slope_intensity = ((dx.abs() + dy.abs()).min(1.0) * 0.1) as f32;
                            color[0] = (color[0] * (1.0 - slope_intensity) + 139.0 * slope_intensity).min(255.0); // Add brown
                            color[1] = (color[1] * (1.0 - slope_intensity) + 90.0 * slope_intensity).min(255.0);
                            color[2] = (color[2] * (1.0 - slope_intensity) + 43.0 * slope_intensity).min(255.0);
                        }
                    }
                    
                    // Check if we're near a biome boundary
                    let mut near_boundary = false;
                    let mut boundary_strength: f32 = 0.0;
                    
                    // Sample neighboring points to detect boundaries
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            if dx == 0 && dy == 0 { continue; }
                            
                            let nx = (terrain_x as i32 + dx) as usize;
                            let ny = (terrain_y as i32 + dy) as usize;
                            
                            if nx < width && ny < height {
                                let neighbor = &map.terrain[ny][nx];
                                
                                // Check for biome differences
                                let biome_diff = match (current_terrain.biome, neighbor.biome) {
                                    (a, b) if a == b => 0.0,
                                    // Water to land transitions
                                    (Biome::Ocean | Biome::DeepOcean | Biome::Shore, 
                                     Biome::Beach | Biome::Plains | Biome::Forest | Biome::Hills) |
                                    (Biome::Beach | Biome::Plains | Biome::Forest | Biome::Hills,
                                     Biome::Ocean | Biome::DeepOcean | Biome::Shore) => 1.0,
                                    // Different land biomes
                                    _ => 0.3,
                                };
                                
                                if biome_diff > 0.0 {
                                    near_boundary = true;
                                    boundary_strength = boundary_strength.max(biome_diff);
                                }
                            }
                        }
                    }
                    
                    // Add subtle noise at boundaries for more natural transitions
                    let mut final_color = color;
                    if near_boundary {
                        // Add Perlin-like noise pattern using position
                        let noise_x = tx * 0.5;
                        let noise_y = ty * 0.5;
                        let noise = ((noise_x * 12.9898 + noise_y * 78.233).sin() * 43758.5453).fract();
                        
                        // Apply subtle color variation
                        let variation = (noise - 0.5) * boundary_strength * 10.0;
                        final_color[0] = (final_color[0] + variation).max(0.0).min(255.0);
                        final_color[1] = (final_color[1] + variation).max(0.0).min(255.0);
                        final_color[2] = (final_color[2] + variation).max(0.0).min(255.0);
                    }
                    
                    // Add coastline detection for water edges
                    let is_water = matches!(current_terrain.biome, 
                        Biome::Ocean | Biome::DeepOcean | Biome::Shore | Biome::Lake);
                    
                    if near_boundary && boundary_strength > 0.8 && is_water {
                        // Darken water edges slightly for coastline effect
                        final_color[0] *= 0.85;
                        final_color[1] *= 0.9;
                        final_color[2] *= 0.95;
                    }
                    
                    let pixel_index = ((py * img_width + px) * 4) as usize;
                    if pixel_index + 3 < pixels.len() {
                        pixels[pixel_index] = final_color[0] as u8;
                        pixels[pixel_index + 1] = final_color[1] as u8;
                        pixels[pixel_index + 2] = final_color[2] as u8;
                        pixels[pixel_index + 3] = 255;
                    }
                }
            }
        }
        
        // Draw rivers with appropriate width for the scale
        for river in &map.rivers {
            for i in 0..river.len() {
                let (x, y) = river[i];
                
                // For small scales (GUI), draw rivers directly without expansion
                // For large scales (CLI), use wider brush
                let brush_range = if scale <= 2 { 0i32..=0 } else { -1i32..=1 };
                
                for dy in brush_range.clone() {
                    for dx in brush_range.clone() {
                        let rx = (x as i32 + dx) as usize;
                        let ry = (y as i32 + dy) as usize;
                        
                        if rx < width && ry < height {
                            // Much brighter, more saturated blue for visibility
                            let river_color = [30, 100, 220, 255];  // Bright saturated blue
                            
                            for sy in 0..scale {
                                for sx in 0..scale {
                                    let px = rx * scale + sx;
                                    let py = ry * scale + sy;
                                    let pixel_index = ((py * img_width + px) * 4) as usize;
                                    
                                    if pixel_index + 3 < pixels.len() {
                                        // For small scales, use full opacity for visibility
                                        if scale <= 2 {
                                            pixels[pixel_index] = river_color[0];
                                            pixels[pixel_index + 1] = river_color[1];
                                            pixels[pixel_index + 2] = river_color[2];
                                        } else {
                                            // For larger scales, use blending
                                            let blend = 0.9;
                                            pixels[pixel_index] = (pixels[pixel_index] as f32 * (1.0 - blend) + river_color[0] as f32 * blend) as u8;
                                            pixels[pixel_index + 1] = (pixels[pixel_index + 1] as f32 * (1.0 - blend) + river_color[1] as f32 * blend) as u8;
                                            pixels[pixel_index + 2] = (pixels[pixel_index + 2] as f32 * (1.0 - blend) + river_color[2] as f32 * blend) as u8;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Draw roads with better visibility
        for road in &map.roads {
            // Darker, more visible colors
            let (road_color, road_width) = match road.road_type.as_str() {
                "highway" => ([40, 40, 45, 230], 2),  // Dark gray, 2 pixels wide
                "road" => ([60, 55, 50, 220], 1),     // Dark brown-gray, 1 pixel
                _ => ([80, 70, 60, 200], 1),          // Brown trail, 1 pixel
            };
            
            // Draw road path
            for i in 0..road.path.len() {
                let (x, y) = road.path[i];
                if x < width && y < height {
                    let base_px = x * scale + scale / 2;
                    let base_py = y * scale + scale / 2;
                    
                    // Draw with specified width
                    for offset in 0..road_width {
                        // Draw main pixel
                        let px = base_px + offset;
                        let py = base_py;
                        let pixel_index = ((py * img_width + px) * 4) as usize;
                        
                        if pixel_index + 3 < pixels.len() {
                            let blend = road_color[3] as f32 / 255.0;
                            pixels[pixel_index] = (pixels[pixel_index] as f32 * (1.0 - blend) + road_color[0] as f32 * blend) as u8;
                            pixels[pixel_index + 1] = (pixels[pixel_index + 1] as f32 * (1.0 - blend) + road_color[1] as f32 * blend) as u8;
                            pixels[pixel_index + 2] = (pixels[pixel_index + 2] as f32 * (1.0 - blend) + road_color[2] as f32 * blend) as u8;
                        }
                        
                        // Also draw perpendicular pixel for 2-pixel highways
                        if road_width == 2 && offset == 0 {
                            let pixel_index_v = (((py + 1) * img_width + px) * 4) as usize;
                            if pixel_index_v + 3 < pixels.len() {
                                let blend = road_color[3] as f32 / 255.0;
                                pixels[pixel_index_v] = (pixels[pixel_index_v] as f32 * (1.0 - blend) + road_color[0] as f32 * blend) as u8;
                                pixels[pixel_index_v + 1] = (pixels[pixel_index_v + 1] as f32 * (1.0 - blend) + road_color[1] as f32 * blend) as u8;
                                pixels[pixel_index_v + 2] = (pixels[pixel_index_v + 2] as f32 * (1.0 - blend) + road_color[2] as f32 * blend) as u8;
                            }
                        }
                    }
                    
                    // Connect to next point with interpolation for smooth curves
                    if i < road.path.len() - 1 {
                        let (next_x, next_y) = road.path[i + 1];
                        let next_px = next_x * scale + scale / 2;
                        let next_py = next_y * scale + scale / 2;
                        
                        // Simple line interpolation
                        let dx = (next_px as i32 - base_px as i32).abs();
                        let dy = (next_py as i32 - base_py as i32).abs();
                        let steps = dx.max(dy) as usize;
                        
                        if steps > 0 {
                            let road_blend = road_color[3] as f32 / 255.0;
                            for step in 1..steps {
                                let t = step as f32 / steps as f32;
                                let interp_x = (base_px as f32 * (1.0 - t) + next_px as f32 * t) as usize;
                                let interp_y = (base_py as f32 * (1.0 - t) + next_py as f32 * t) as usize;
                                
                                // Draw main line
                                for offset in 0..road_width {
                                    let px = interp_x + offset;
                                    let py = interp_y;
                                    let interp_idx = ((py * img_width + px) * 4) as usize;
                                    
                                    if interp_idx + 3 < pixels.len() {
                                        pixels[interp_idx] = (pixels[interp_idx] as f32 * (1.0 - road_blend) + road_color[0] as f32 * road_blend) as u8;
                                        pixels[interp_idx + 1] = (pixels[interp_idx + 1] as f32 * (1.0 - road_blend) + road_color[1] as f32 * road_blend) as u8;
                                        pixels[interp_idx + 2] = (pixels[interp_idx + 2] as f32 * (1.0 - road_blend) + road_color[2] as f32 * road_blend) as u8;
                                    }
                                    
                                    // Perpendicular pixel for highways
                                    if road_width == 2 && offset == 0 {
                                        let interp_idx_v = (((py + 1) * img_width + px) * 4) as usize;
                                        if interp_idx_v + 3 < pixels.len() {
                                            pixels[interp_idx_v] = (pixels[interp_idx_v] as f32 * (1.0 - road_blend) + road_color[0] as f32 * road_blend) as u8;
                                            pixels[interp_idx_v + 1] = (pixels[interp_idx_v + 1] as f32 * (1.0 - road_blend) + road_color[1] as f32 * road_blend) as u8;
                                            pixels[interp_idx_v + 2] = (pixels[interp_idx_v + 2] as f32 * (1.0 - road_blend) + road_color[2] as f32 * road_blend) as u8;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Draw cities as round dots with circles for large cities
        for city in &map.cities {
            let cx = city.x * scale + scale / 2;
            let cy = city.y * scale + scale / 2;
            
            // Determine if it's a large city that needs a circle
            let is_large_city = city.population > 100000;
            
            // City dot sizes - scaled based on tile size for visibility
            let size_factor = (scale as f32 / 10.0).max(0.5); // Scale relative to 10px baseline
            let dot_radius = if city.population > 250000 { 
                (12.0 * size_factor) as usize  // Major cities
            } else if city.population > 100000 { 
                (9.0 * size_factor) as usize   // Large cities
            } else { 
                (6.0 * size_factor) as usize   // Towns
            };
            
            // Draw circle around large cities first
            if is_large_city {
                let circle_radius = dot_radius + 3; // Circle 3 pixels larger than dot
                
                // Draw circle outline using Bresenham-style approach
                for dy in -(circle_radius as i32 + 1)..=(circle_radius as i32 + 1) {
                    for dx in -(circle_radius as i32 + 1)..=(circle_radius as i32 + 1) {
                        let dist_sq = dx * dx + dy * dy;
                        let outer_radius_sq = ((circle_radius + 1) * (circle_radius + 1)) as i32;
                        let inner_radius_sq = ((circle_radius - 1) * (circle_radius - 1)) as i32;
                        
                        // Draw if we're in the circle ring (not inside, not outside)
                        if dist_sq as i32 <= outer_radius_sq && dist_sq as i32 >= inner_radius_sq {
                            let px = (cx as i32 + dx) as usize;
                            let py = (cy as i32 + dy) as usize;
                            let pixel_index = ((py * img_width + px) * 4) as usize;
                            
                            if pixel_index + 3 < pixels.len() && px < img_width && py < img_height {
                                pixels[pixel_index] = 20;  // Black circle
                                pixels[pixel_index + 1] = 20;
                                pixels[pixel_index + 2] = 20;
                                pixels[pixel_index + 3] = 255;
                            }
                        }
                    }
                }
            }
            
            // Draw solid round dot for city
            for dy in -(dot_radius as i32)..=(dot_radius as i32) {
                for dx in -(dot_radius as i32)..=(dot_radius as i32) {
                    let dist_sq = dx * dx + dy * dy;
                    if dist_sq <= (dot_radius * dot_radius) as i32 {
                        let px = (cx as i32 + dx) as usize;
                        let py = (cy as i32 + dy) as usize;
                        let pixel_index = ((py * img_width + px) * 4) as usize;
                        
                        if pixel_index + 3 < pixels.len() && px < img_width && py < img_height {
                            // Use contrasting colors
                            if city.population > 250000 {
                                // Major cities - red dot
                                pixels[pixel_index] = 220;
                                pixels[pixel_index + 1] = 20;
                                pixels[pixel_index + 2] = 20;
                            } else if city.population > 100000 {
                                // Large cities - dark red dot
                                pixels[pixel_index] = 180;
                                pixels[pixel_index + 1] = 40;
                                pixels[pixel_index + 2] = 40;
                            } else {
                                // Towns - black dot
                                pixels[pixel_index] = 20;
                                pixels[pixel_index + 1] = 20;
                                pixels[pixel_index + 2] = 20;
                            }
                            pixels[pixel_index + 3] = 255;
                        }
                    }
                }
            }
        }
        
        pixels
    }
    
    /// Renders terrain map to an image for PNG export
    pub fn render_to_image(map: &TerrainMap, scale: u32) -> RgbImage {
        let width = map.width as u32 * scale;
        let height = map.height as u32 * scale;
        let mut img = ImageBuffer::new(width, height);
        
        // Get the pixel data
        let pixels = Self::render_to_pixels(map, map.width, map.height, scale as usize);
        
        // Convert to RGB image
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                if idx + 2 < pixels.len() {
                    img.put_pixel(x, y, Rgb([pixels[idx], pixels[idx + 1], pixels[idx + 2]]));
                }
            }
        }
        
        img
    }
}
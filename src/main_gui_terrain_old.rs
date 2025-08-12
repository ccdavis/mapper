mod map_generator;
mod terrain_generator;
mod terrain_renderer;

use terrain_generator::{TerrainGenerator, TerrainMap};
use terrain_renderer::TerrainRenderer;
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};
use std::time::SystemTime;
use rusttype::{Font, Scale};
use imageproc::drawing::draw_text_mut;
use image::{ImageBuffer, Rgb};

slint::include_modules!();

fn generate_terrain_image(map: &TerrainMap) -> Image {
    let width = map.width;
    let height = map.height;
    let scale = 2; // Tiny tiles - each tile is only 2x2 pixels for maximum map visibility
    let img_width = width * scale;
    let img_height = height * scale;
    
    let mut pixel_buffer = SharedPixelBuffer::<Rgba8Pixel>::new(img_width as u32, img_height as u32);
    let pixels = pixel_buffer.make_mut_bytes();
    
    // Helper function to get interpolated color
    let get_terrain_color = |x: usize, y: usize| -> [f32; 3] {
        if x < width && y < height {
            let terrain_point = &map.terrain[y][x];
            let base_color = Biome::elevation_color(terrain_point.elevation);
            let biome_color = terrain_point.biome.color();
            let blend_factor = 0.7;
            [
                base_color[0] as f32 * (1.0 - blend_factor) + biome_color[0] as f32 * blend_factor,
                base_color[1] as f32 * (1.0 - blend_factor) + biome_color[1] as f32 * blend_factor,
                base_color[2] as f32 * (1.0 - blend_factor) + biome_color[2] as f32 * blend_factor,
            ]
        } else {
            [0.0, 0.0, 0.0]
        }
    };
    
    // Render terrain with anti-aliasing for smoother edges
    for y in 0..height {
        for x in 0..width {
            let current_terrain = &map.terrain[y][x];
            let base_color = get_terrain_color(x, y);
            
            // Check if this is an edge tile (different biome neighbors)
            let mut is_edge = false;
            let mut edge_colors = Vec::new();
            edge_colors.push(base_color);
            
            // Check neighbors for different biomes
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 { continue; }
                    let nx = (x as i32 + dx) as usize;
                    let ny = (y as i32 + dy) as usize;
                    if nx < width && ny < height {
                        let neighbor = &map.terrain[ny][nx];
                        // Check if it's a different terrain type (for anti-aliasing)
                        let is_water_current = matches!(current_terrain.biome, 
                            Biome::Ocean | Biome::DeepOcean | Biome::Shore | Biome::Lake);
                        let is_water_neighbor = matches!(neighbor.biome, 
                            Biome::Ocean | Biome::DeepOcean | Biome::Shore | Biome::Lake);
                        
                        if is_water_current != is_water_neighbor {
                            is_edge = true;
                            edge_colors.push(get_terrain_color(nx, ny));
                        }
                    }
                }
            }
            
            // Determine if this is a coastline
            let mut is_coastline = false;
            if is_edge {
                // Check if this is specifically a water-land boundary
                let is_water = matches!(current_terrain.biome, 
                    Biome::Ocean | Biome::DeepOcean | Biome::Shore | Biome::Lake);
                let has_land_neighbor = edge_colors.len() > 1;
                is_coastline = is_water || has_land_neighbor;
            }
            
            // Don't modify entire tiles for coastlines, just mark them
            let final_color = if is_edge && edge_colors.len() > 1 && !is_coastline {
                // Regular anti-aliasing for non-coastline edges
                let mut r = 0.0;
                let mut g = 0.0;
                let mut b = 0.0;
                for color in &edge_colors {
                    r += color[0];
                    g += color[1];
                    b += color[2];
                }
                let count = edge_colors.len() as f32;
                [
                    (r / count) as u8,
                    (g / count) as u8,
                    (b / count) as u8,
                    255,
                ]
            } else {
                // Normal terrain color
                [
                    base_color[0] as u8,
                    base_color[1] as u8,
                    base_color[2] as u8,
                    255,
                ]
            };
            
            // Fill the 2x2 pixel tile
            for sy in 0..scale {
                for sx in 0..scale {
                    let px = x * scale + sx;
                    let py = y * scale + sy;
                    let pixel_index = ((py * img_width + px) * 4) as usize;
                    if pixel_index + 3 < pixels.len() {
                        // Normal rendering
                        if is_edge && scale == 2 && !is_coastline {
                            // Soften non-coastline edges within the 2x2 tile
                            let factor = if (sx == 0 && sy == 0) || (sx == 1 && sy == 1) {
                                0.9  // Slightly blend corners
                            } else {
                                1.0  // Full color for other pixels
                            };
                            pixels[pixel_index] = (final_color[0] as f32 * factor) as u8;
                            pixels[pixel_index + 1] = (final_color[1] as f32 * factor) as u8;
                            pixels[pixel_index + 2] = (final_color[2] as f32 * factor) as u8;
                            pixels[pixel_index + 3] = 255;
                        } else {
                            pixels[pixel_index] = final_color[0];
                            pixels[pixel_index + 1] = final_color[1];
                            pixels[pixel_index + 2] = final_color[2];
                            pixels[pixel_index + 3] = final_color[3];
                            
                            // Draw single pixel coastline on the edge
                            if is_coastline && scale == 2 {
                                // Only draw on the actual edge pixels
                                if (sx == 1 && edge_colors.len() > 0) || (sy == 1 && edge_colors.len() > 0) {
                                    // Dark blue coastline
                                    pixels[pixel_index] = 10;
                                    pixels[pixel_index + 1] = 40;
                                    pixels[pixel_index + 2] = 100;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Draw rivers with increased width for better visibility
    for river in &map.rivers {
        for i in 0..river.len() {
            let (x, y) = river[i];
            
            // Draw with a wider brush for visibility
            for dy in -2i32..=2 {
                for dx in -2i32..=2 {
                    let rx = (x as i32 + dx) as usize;
                    let ry = (y as i32 + dy) as usize;
                    
                    if rx < width && ry < height {
                        let river_color = Biome::River.color();
                        
                        // Anti-aliasing: fade at edges based on distance
                        let dist = ((dx * dx + dy * dy) as f32).sqrt();
                        let alpha = if dist <= 1.0 { 230 } else if dist <= 2.0 { 150 } else { 100 };
                        
                        for sy in 0..scale {
                            for sx in 0..scale {
                                let px = rx * scale + sx;
                                let py = ry * scale + sy;
                                let pixel_index = ((py * img_width + px) * 4) as usize;
                                
                                if pixel_index + 3 < pixels.len() {
                                    // Blend with existing color
                                    let blend = alpha as f32 / 255.0;
                                    pixels[pixel_index] = ((pixels[pixel_index] as f32 * (1.0 - blend) + river_color[0] as f32 * blend) as u8);
                                    pixels[pixel_index + 1] = ((pixels[pixel_index + 1] as f32 * (1.0 - blend) + river_color[1] as f32 * blend) as u8);
                                    pixels[pixel_index + 2] = ((pixels[pixel_index + 2] as f32 * (1.0 - blend) + river_color[2] as f32 * blend) as u8);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Draw roads (single pixel lines)
    for road in &map.roads {
        let road_color = match road.road_type.as_str() {
            "highway" => [80, 80, 80, 200],
            "road" => [110, 100, 90, 180],
            _ => [140, 130, 110, 160],
        };
        
        // Draw road path with single pixels
        for i in 0..road.path.len() {
            let (x, y) = road.path[i];
            if x < width && y < height {
                // Draw single pixel at the center of each tile
                let px = x * scale + scale / 2;
                let py = y * scale + scale / 2;
                let pixel_index = ((py * img_width + px) * 4) as usize;
                
                if pixel_index + 3 < pixels.len() {
                    // Simple blend for road
                    let blend = road_color[3] as f32 / 255.0;
                    pixels[pixel_index] = (pixels[pixel_index] as f32 * (1.0 - blend) + road_color[0] as f32 * blend) as u8;
                    pixels[pixel_index + 1] = (pixels[pixel_index + 1] as f32 * (1.0 - blend) + road_color[1] as f32 * blend) as u8;
                    pixels[pixel_index + 2] = (pixels[pixel_index + 2] as f32 * (1.0 - blend) + road_color[2] as f32 * blend) as u8;
                }
                
                // Connect to next point with interpolation for smooth curves
                if i < road.path.len() - 1 {
                    let (next_x, next_y) = road.path[i + 1];
                    let next_px = next_x * scale + scale / 2;
                    let next_py = next_y * scale + scale / 2;
                    
                    // Simple line interpolation
                    let dx = (next_px as i32 - px as i32).abs();
                    let dy = (next_py as i32 - py as i32).abs();
                    let steps = dx.max(dy) as usize;
                    
                    if steps > 0 {
                        let road_blend = road_color[3] as f32 / 255.0;
                        for step in 1..steps {
                            let t = step as f32 / steps as f32;
                            let interp_x = (px as f32 * (1.0 - t) + next_px as f32 * t) as usize;
                            let interp_y = (py as f32 * (1.0 - t) + next_py as f32 * t) as usize;
                            let interp_idx = ((interp_y * img_width + interp_x) * 4) as usize;
                            
                            if interp_idx + 3 < pixels.len() {
                                pixels[interp_idx] = (pixels[interp_idx] as f32 * (1.0 - road_blend) + road_color[0] as f32 * road_blend) as u8;
                                pixels[interp_idx + 1] = (pixels[interp_idx + 1] as f32 * (1.0 - road_blend) + road_color[1] as f32 * road_blend) as u8;
                                pixels[interp_idx + 2] = (pixels[interp_idx + 2] as f32 * (1.0 - road_blend) + road_color[2] as f32 * road_blend) as u8;
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Draw cities (much smaller markers)
    for city in &map.cities {
        let cx = city.x * scale + scale / 2;
        let cy = city.y * scale + scale / 2;
        
        // Much smaller city sizes - just a few pixels
        let city_radius_pixels = if city.population > 250000 { 
            3  // Major cities - 3 pixel radius
        } else if city.population > 100000 { 
            2  // Medium cities - 2 pixel radius
        } else { 
            1  // Towns - 1 pixel radius
        };
        
        let city_color = [40, 40, 40, 255]; // Dark gray for cities
        let border_color = [20, 20, 20, 255]; // Darker border
        
        // Draw city marker as a small circle
        for dy in -(city_radius_pixels as i32)..=(city_radius_pixels as i32) {
            for dx in -(city_radius_pixels as i32)..=(city_radius_pixels as i32) {
                let px = (cx as i32 + dx) as usize;
                let py = (cy as i32 + dy) as usize;
                
                if px < img_width && py < img_height {
                    let dist = ((dx * dx + dy * dy) as f32).sqrt();
                    if dist <= city_radius_pixels as f32 {
                        let pixel_index = ((py * img_width + px) * 4) as usize;
                        
                        if pixel_index + 3 < pixels.len() {
                            // Draw with border for larger cities
                            if city_radius_pixels > 1 && dist > city_radius_pixels as f32 - 1.0 {
                                pixels[pixel_index] = border_color[0];
                                pixels[pixel_index + 1] = border_color[1];
                                pixels[pixel_index + 2] = border_color[2];
                                pixels[pixel_index + 3] = border_color[3];
                            } else {
                                pixels[pixel_index] = city_color[0];
                                pixels[pixel_index + 1] = city_color[1];
                                pixels[pixel_index + 2] = city_color[2];
                                pixels[pixel_index + 3] = city_color[3];
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Convert to RGB image for text rendering
    let mut rgb_img: RgbImage = ImageBuffer::new(img_width as u32, img_height as u32);
    for y in 0..img_height {
        for x in 0..img_width {
            let idx = ((y * img_width + x) * 4) as usize;
            if idx + 2 < pixels.len() {
                rgb_img.put_pixel(x as u32, y as u32, Rgb([pixels[idx], pixels[idx + 1], pixels[idx + 2]]));
            }
        }
    }
    
    // Load font for text rendering
    let font_data = include_bytes!("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf");
    let font = Font::try_from_bytes(font_data as &[u8]).unwrap();
    
    // Draw city labels
    for city in &map.cities {
        let x = (city.x * scale) + scale;
        let y = (city.y * scale) - scale / 2;
        
        let text_scale = Scale::uniform(14.0 * (scale as f32 / 20.0));
        
        // Draw black outline for visibility
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx != 0 || dy != 0 {
                    draw_text_mut(
                        &mut rgb_img,
                        Rgb([0, 0, 0]),
                        (x as i32 + dx),
                        (y as i32 + dy),
                        text_scale,
                        &font,
                        &city.name
                    );
                }
            }
        }
        
        // Draw white text
        draw_text_mut(
            &mut rgb_img,
            Rgb([255, 255, 255]),
            x as i32,
            y as i32,
            text_scale,
            &font,
            &city.name
        );
        
        // Draw population for large cities
        if city.population > 100000 {
            let pop_text = format!("({}k)", city.population / 1000);
            let pop_scale = Scale::uniform(10.0 * (scale as f32 / 20.0));
            
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx != 0 || dy != 0 {
                        draw_text_mut(
                            &mut rgb_img,
                            Rgb([0, 0, 0]),
                            (x as i32 + dx),
                            (y as i32 + dy + (14.0 * scale as f32 / 20.0) as i32),
                            pop_scale,
                            &font,
                            &pop_text
                        );
                    }
                }
            }
            
            draw_text_mut(
                &mut rgb_img,
                Rgb([200, 200, 200]),
                x as i32,
                (y + (14.0 * scale as f32 / 20.0) as usize) as i32,
                pop_scale,
                &font,
                &pop_text
            );
        }
    }
    
    // Draw geographic feature labels
    for label in &map.labels {
        let x = (label.x * scale as f32) as usize;
        let y = (label.y * scale as f32) as usize;
        
        let text_color = match label.feature_type.as_str() {
            "ocean" => Rgb([150, 200, 255]),
            "mountains" => Rgb([150, 150, 150]),
            "forest" => Rgb([100, 200, 100]),
            "swamp" => Rgb([150, 180, 150]),
            "river" => Rgb([100, 150, 255]),
            _ => Rgb([200, 200, 200]),
        };
        
        let label_scale = Scale::uniform(16.0 * (scale as f32 / 20.0));
        
        // Draw black outline
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx != 0 || dy != 0 {
                    draw_text_mut(
                        &mut rgb_img,
                        Rgb([0, 0, 0]),
                        (x as i32 + dx),
                        (y as i32 + dy),
                        label_scale,
                        &font,
                        &label.name
                    );
                }
            }
        }
        
        draw_text_mut(
            &mut rgb_img,
            text_color,
            x as i32,
            y as i32,
            label_scale,
            &font,
            &label.name
        );
    }
    
    // Draw road labels
    for road in &map.roads {
        if !road.path.is_empty() {
            let mid_idx = road.path.len() / 2;
            let (mid_x, mid_y) = road.path[mid_idx];
            
            let x = mid_x * scale;
            let y = mid_y * scale;
            
            let road_scale = Scale::uniform(12.0 * (scale as f32 / 20.0));
            
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx != 0 || dy != 0 {
                        draw_text_mut(
                            &mut rgb_img,
                            Rgb([0, 0, 0]),
                            (x as i32 + dx),
                            (y as i32 + dy),
                            road_scale,
                            &font,
                            &road.name
                        );
                    }
                }
            }
            
            draw_text_mut(
                &mut rgb_img,
                Rgb([180, 180, 180]),
                x as i32,
                y as i32,
                road_scale,
                &font,
                &road.name
            );
        }
    }
    
    // Draw bridge labels
    for bridge in &map.bridges {
        let x = bridge.x * scale;
        let y = bridge.y * scale - scale / 2;
        
        let bridge_scale = Scale::uniform(10.0 * (scale as f32 / 20.0));
        
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx != 0 || dy != 0 {
                    draw_text_mut(
                        &mut rgb_img,
                        Rgb([0, 0, 0]),
                        (x as i32 + dx),
                        (y as i32 + dy),
                        bridge_scale,
                        &font,
                        &bridge.name
                    );
                }
            }
        }
        
        draw_text_mut(
            &mut rgb_img,
            Rgb([150, 200, 255]),
            x as i32,
            y as i32,
            bridge_scale,
            &font,
            &bridge.name
        );
    }
    
    // Convert back to RGBA for Slint
    let mut final_buffer = SharedPixelBuffer::<Rgba8Pixel>::new(img_width as u32, img_height as u32);
    let final_pixels = final_buffer.make_mut_bytes();
    
    for y in 0..img_height {
        for x in 0..img_width {
            let pixel = rgb_img.get_pixel(x as u32, y as u32);
            let idx = ((y * img_width + x) * 4) as usize;
            if idx + 3 < final_pixels.len() {
                final_pixels[idx] = pixel[0];
                final_pixels[idx + 1] = pixel[1];
                final_pixels[idx + 2] = pixel[2];
                final_pixels[idx + 3] = 255;
            }
        }
    }
    
    Image::from_rgba8(final_buffer)
}

fn generate_labels_text(map: &TerrainMap) -> String {
    let mut labels_text = String::new();
    
    // Add cities info
    if !map.cities.is_empty() {
        labels_text.push_str("Cities:\n");
        for city in map.cities.iter().take(5) {
            labels_text.push_str(&format!("  {} (pop: {})\n", city.name, city.population));
        }
        labels_text.push_str("\n");
    }
    
    // Add roads info
    if !map.roads.is_empty() {
        labels_text.push_str("Major Roads:\n");
        for road in map.roads.iter().take(3) {
            labels_text.push_str(&format!("  {}\n", road.name));
        }
        labels_text.push_str("\n");
    }
    
    // Add geographic features
    labels_text.push_str("Geographic Features:\n");
    for label in &map.labels {
        labels_text.push_str(&format!("  {}: {}\n", 
            label.feature_type, label.name));
    }
    
    if labels_text.is_empty() {
        "No features generated".to_string()
    } else {
        labels_text
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = MapperWindow::new()?;
    
    let ui_handle = ui.as_weak();
    ui.on_menu_start(move || {
        let ui = ui_handle.unwrap();
        
        // Generate terrain with current timestamp as seed
        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32;
        
        let mut generator = TerrainGenerator::new(seed);
        let terrain_map = generator.generate(1600, 1000); // Very large map as requested
        
        // Convert to image
        let map_image = generate_terrain_image(&terrain_map);
        
        // Generate labels info
        let labels_text = generate_labels_text(&terrain_map);
        
        // Update UI
        ui.set_map_image(map_image);
        ui.set_has_map(true);
        ui.set_map_status(format!("Terrain generated: {} cities, {} roads, {} rivers\n\n{}", 
            terrain_map.cities.len(),
            terrain_map.roads.len(),
            terrain_map.rivers.len(),
            labels_text).into());
    });
    
    let ui_handle = ui.as_weak();
    ui.on_menu_exit(move || {
        let ui = ui_handle.unwrap();
        ui.hide().unwrap();
        std::process::exit(0);
    });
    
    let ui_handle = ui.as_weak();
    ui.on_menu_about(move || {
        let ui = ui_handle.unwrap();
        ui.set_map_status("Mapper v0.1.0\nRealistic Terrain Generator\nUsing Perlin noise and biome simulation".into());
    });
    
    ui.run()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_terrain_generation() {
        let mut generator = TerrainGenerator::new(42);
        let map = generator.generate(20, 20);
        
        assert_eq!(map.width, 20);
        assert_eq!(map.height, 20);
        assert_eq!(map.terrain.len(), 20);
        assert_eq!(map.terrain[0].len(), 20);
        
        // Check that we have varied terrain
        let mut biomes = std::collections::HashSet::new();
        for row in &map.terrain {
            for point in row {
                biomes.insert(format!("{:?}", point.biome));
            }
        }
        assert!(biomes.len() > 1, "Should have multiple biome types");
    }
    
    #[test]
    fn test_terrain_image_generation() {
        let mut generator = TerrainGenerator::new(42);
        let map = generator.generate(10, 10);
        let image = generate_terrain_image(&map);
        
        // Basic validation that image was created
        assert_eq!(format!("{:?}", image).is_empty(), false);
    }
}
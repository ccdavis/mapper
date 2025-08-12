mod map_generator;
mod terrain_generator;
mod terrain_renderer;

use terrain_generator::{TerrainGenerator, TerrainMap, Biome};
use terrain_renderer::TerrainRenderer;
use std::io::{self, Write};
use std::time::SystemTime;
use image::{ImageBuffer, Rgb, RgbImage};
use rusttype::{Font, Scale};
use imageproc::drawing::draw_text_mut;

fn print_terrain_ascii(map: &TerrainMap) {
    // ASCII representation with more nuanced characters
    for y in 0..map.height {
        for x in 0..map.width {
            let point = &map.terrain[y][x];
            
            // Check if this is a river point
            let is_river = map.rivers.iter().any(|river| {
                river.iter().any(|&(rx, ry)| rx == x && ry == y)
            });
            
            let ch = if is_river {
                '~' // River
            } else {
                match point.biome {
                    Biome::DeepOcean => '≈',
                    Biome::Ocean => '~',
                    Biome::Shore => '-',
                    Biome::Beach => '.',
                    Biome::Plains => ',',
                    Biome::Forest => '♣',
                    Biome::Hills => 'n',
                    Biome::Mountains => '▲',
                    Biome::SnowPeaks => '△',
                    Biome::River => '~',
                    Biome::Lake => 'o',
                    Biome::Swamp => '%',
                    Biome::Desert => '=',
                }
            };
            
            // Color based on biome (ANSI colors)
            let color_code = match point.biome {
                Biome::DeepOcean => "\x1b[34m",   // Blue
                Biome::Ocean => "\x1b[36m",       // Cyan
                Biome::Shore => "\x1b[96m",       // Light cyan
                Biome::Beach => "\x1b[93m",       // Yellow
                Biome::Plains => "\x1b[92m",      // Light green
                Biome::Forest => "\x1b[32m",      // Green
                Biome::Hills => "\x1b[33m",       // Brown/yellow
                Biome::Mountains => "\x1b[90m",   // Gray
                Biome::SnowPeaks => "\x1b[97m",   // White
                Biome::River => "\x1b[94m",       // Light blue
                Biome::Lake => "\x1b[36m",        // Cyan
                Biome::Swamp => "\x1b[35m",       // Magenta
                Biome::Desert => "\x1b[93m",      // Yellow
            };
            
            print!("{}{}\x1b[0m", color_code, ch);
        }
        println!();
    }
}

fn print_terrain_info(map: &TerrainMap) {
    println!("\n\x1b[1mTerrain Features:\x1b[0m");
    println!("═══════════════════════════════");
    
    // Count biomes
    let mut biome_counts = std::collections::HashMap::new();
    for row in &map.terrain {
        for point in row {
            *biome_counts.entry(format!("{:?}", point.biome)).or_insert(0) += 1;
        }
    }
    
    println!("\n\x1b[1mBiome Distribution:\x1b[0m");
    for (biome, count) in biome_counts.iter() {
        let percentage = (*count as f32 / (map.width * map.height) as f32) * 100.0;
        println!("  {} - {:.1}%", biome, percentage);
    }
    
    println!("\n\x1b[1mRivers:\x1b[0m {} generated", map.rivers.len());
    
    println!("\n\x1b[1mCities:\x1b[0m {} cities", map.cities.len());
    for (i, city) in map.cities.iter().enumerate().take(5) {
        println!("  • {} - Population: {}", city.name, city.population);
    }
    
    println!("\n\x1b[1mRoads:\x1b[0m {} roads", map.roads.len());
    for road in map.roads.iter().take(3) {
        let bridge_info = if !road.bridges.is_empty() {
            format!(" with {} bridge(s)", road.bridges.len())
        } else {
            String::new()
        };
        println!("  • {} ({}){}", road.name, road.road_type, bridge_info);
    }
    
    if !map.bridges.is_empty() {
        println!("\n\x1b[1mBridges:\x1b[0m {} bridges", map.bridges.len());
        for bridge in map.bridges.iter().take(5) {
            println!("  • {}", bridge.name);
        }
    }
    
    println!("\n\x1b[1mNamed Locations:\x1b[0m");
    for label in &map.labels {
        println!("  • {} - {} (at {:.0}, {:.0})", 
            label.feature_type, label.name, label.x, label.y);
    }
    
    println!("\n\x1b[1mLegend:\x1b[0m");
    println!("  \x1b[34m≈\x1b[0m Deep Ocean    \x1b[36m~\x1b[0m Ocean       \x1b[96m-\x1b[0m Shore");
    println!("  \x1b[93m.\x1b[0m Beach        \x1b[92m,\x1b[0m Plains      \x1b[32m♣\x1b[0m Forest");
    println!("  \x1b[33mn\x1b[0m Hills        \x1b[90m▲\x1b[0m Mountains   \x1b[97m△\x1b[0m Snow Peaks");
    println!("  \x1b[94m~\x1b[0m Rivers");
}

fn save_terrain_png(map: &TerrainMap, filename: &str, base_scale: u32) -> Result<(), image::ImageError> {
    // Much higher resolution with bilinear interpolation
    let scale = base_scale * 4; // 4x higher resolution
    let width = (map.width as u32) * scale;
    let height = (map.height as u32) * scale;
    
    let mut img: RgbImage = ImageBuffer::new(width, height);
    
    // Helper function to get interpolated color
    let get_terrain_color = |x: usize, y: usize| -> [f32; 3] {
        if x < map.width && y < map.height {
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
    
    // Render terrain with coastline detection
    for y in 0..map.height {
        for x in 0..map.width {
            let current_terrain = &map.terrain[y][x];
            let base_color = get_terrain_color(x, y);
            
            // Check if this is a coastline tile
            let mut is_coastline = false;
            let is_water = matches!(current_terrain.biome, 
                Biome::Ocean | Biome::DeepOcean | Biome::Shore | Biome::Lake);
            
            // Check neighbors for land/water boundary
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 { continue; }
                    let nx = (x as i32 + dx) as usize;
                    let ny = (y as i32 + dy) as usize;
                    if nx < map.width && ny < map.height {
                        let neighbor = &map.terrain[ny][nx];
                        let neighbor_is_water = matches!(neighbor.biome,
                            Biome::Ocean | Biome::DeepOcean | Biome::Shore | Biome::Lake);
                        if is_water != neighbor_is_water {
                            is_coastline = true;
                            break;
                        }
                    }
                }
                if is_coastline { break; }
            }
            
            // Determine final color
            let final_color = if is_coastline {
                if is_water {
                    // Water side of coastline - darker blue outline
                    [20, 60, 110]
                } else {
                    // Land side of coastline - sandy beach color
                    [200, 180, 140]
                }
            } else {
                [
                    base_color[0] as u8,
                    base_color[1] as u8, 
                    base_color[2] as u8,
                ]
            };
            
            // Fill all pixels for this cell
            for sy in 0..scale {
                for sx in 0..scale {
                    let px = x as u32 * scale + sx;
                    let py = y as u32 * scale + sy;
                    img.put_pixel(px, py, Rgb(final_color));
                }
            }
        }
    }
    
    // Draw rivers with anti-aliasing and increased width
    for river in &map.rivers {
        for &(x, y) in river {
            if x < map.width && y < map.height {
                let river_color = Biome::River.color();
                
                // Draw with a wider brush for better visibility
                for dy in -2i32..=2 {
                    for dx in -2i32..=2 {
                        let rx = (x as i32 + dx) as u32;
                        let ry = (y as i32 + dy) as u32;
                        
                        if rx < map.width as u32 && ry < map.height as u32 {
                            // Anti-aliasing: fade at edges
                            let dist = ((dx * dx + dy * dy) as f32).sqrt();
                            let alpha = if dist <= 1.0 { 0.9 } else if dist <= 2.0 { 0.5 } else { 0.3 };
                            
                            for sy in 0..scale {
                                for sx in 0..scale {
                                    let px = rx * scale + sx;
                                    let py = ry * scale + sy;
                                    
                                    if px < width && py < height {
                                        let existing = img.get_pixel(px, py);
                                        let blended = Rgb([
                                            (existing[0] as f32 * (1.0 - alpha) + river_color[0] as f32 * alpha) as u8,
                                            (existing[1] as f32 * (1.0 - alpha) + river_color[1] as f32 * alpha) as u8,
                                            (existing[2] as f32 * (1.0 - alpha) + river_color[2] as f32 * alpha) as u8,
                                        ]);
                                        img.put_pixel(px, py, blended);
                                    }
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
            "highway" => [80, 80, 80],
            "road" => [110, 100, 90],
            _ => [140, 130, 110],
        };
        
        // Draw road path with single pixels
        for i in 0..road.path.len() {
            let (x, y) = road.path[i];
            if x < map.width && y < map.height {
                // Draw single pixel at center of tile
                let px = (x as u32) * scale + scale / 2;
                let py = (y as u32) * scale + scale / 2;
                
                if px < width && py < height {
                    let existing = img.get_pixel(px, py);
                    let blended = Rgb([
                        (existing[0] as f32 * 0.2 + road_color[0] as f32 * 0.8) as u8,
                        (existing[1] as f32 * 0.2 + road_color[1] as f32 * 0.8) as u8,
                        (existing[2] as f32 * 0.2 + road_color[2] as f32 * 0.8) as u8,
                    ]);
                    img.put_pixel(px, py, blended);
                }
                
                // Connect to next point with interpolation for smooth curves
                if i < road.path.len() - 1 {
                    let (next_x, next_y) = road.path[i + 1];
                    let next_px = (next_x as u32) * scale + scale / 2;
                    let next_py = (next_y as u32) * scale + scale / 2;
                    
                    // Simple line interpolation
                    let dx = (next_px as i32 - px as i32).abs() as u32;
                    let dy = (next_py as i32 - py as i32).abs() as u32;
                    let steps = dx.max(dy);
                    
                    if steps > 0 {
                        for step in 1..steps {
                            let t = step as f32 / steps as f32;
                            let interp_x = (px as f32 * (1.0 - t) + next_px as f32 * t) as u32;
                            let interp_y = (py as f32 * (1.0 - t) + next_py as f32 * t) as u32;
                            
                            if interp_x < width && interp_y < height {
                                let existing = img.get_pixel(interp_x, interp_y);
                                let blended = Rgb([
                                    (existing[0] as f32 * 0.2 + road_color[0] as f32 * 0.8) as u8,
                                    (existing[1] as f32 * 0.2 + road_color[1] as f32 * 0.8) as u8,
                                    (existing[2] as f32 * 0.2 + road_color[2] as f32 * 0.8) as u8,
                                ]);
                                img.put_pixel(interp_x, interp_y, blended);
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Draw cities (much smaller markers)
    for city in &map.cities {
        let x = (city.x as u32) * scale;
        let y = (city.y as u32) * scale;
        
        // Much smaller city sizes - just a few pixels
        let city_radius_pixels = if city.population > 250000 { 
            3  // Major cities - 3 pixel radius
        } else if city.population > 100000 { 
            2  // Medium cities - 2 pixel radius
        } else { 
            1  // Towns - 1 pixel radius
        };
        
        let city_color = Rgb([40, 40, 40]); // Dark gray for cities
        let city_border = Rgb([20, 20, 20]); // Darker border
        
        // Draw city marker as a small circle
        let center_x = x + scale / 2;
        let center_y = y + scale / 2;
        
        for dy in -(city_radius_pixels as i32)..=(city_radius_pixels as i32) {
            for dx in -(city_radius_pixels as i32)..=(city_radius_pixels as i32) {
                let px = (center_x as i32 + dx) as u32;
                let py = (center_y as i32 + dy) as u32;
                
                if px < width && py < height {
                    let dist = ((dx as f32).powi(2) + (dy as f32).powi(2)).sqrt();
                    
                    if dist <= city_radius_pixels as f32 {
                        // Draw border for larger cities
                        if city_radius_pixels > 1 && dist > city_radius_pixels as f32 - 1.0 {
                            img.put_pixel(px, py, city_border);
                        } else {
                            img.put_pixel(px, py, city_color);
                        }
                    }
                }
            }
        }
    }
    
    // Load font for text rendering
    let font_data = include_bytes!("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf");
    let font = Font::try_from_bytes(font_data as &[u8]).unwrap();
    
    // Draw city labels
    for city in &map.cities {
        let x = (city.x as u32) * scale + scale;
        let y = (city.y as u32) * scale - scale / 2;
        
        // Draw city name with white text and black outline for visibility
        let text_scale = Scale::uniform(14.0 * (scale as f32 / 20.0));
        
        // Draw black outline
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx != 0 || dy != 0 {
                    draw_text_mut(
                        &mut img,
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
            &mut img,
            Rgb([255, 255, 255]),
            x as i32,
            y as i32,
            text_scale,
            &font,
            &city.name
        );
        
        // Draw population if large enough
        if city.population > 100000 {
            let pop_text = format!("({}k)", city.population / 1000);
            let pop_scale = Scale::uniform(10.0 * (scale as f32 / 20.0));
            
            // Draw black outline
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx != 0 || dy != 0 {
                        draw_text_mut(
                            &mut img,
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
                &mut img,
                Rgb([200, 200, 200]),
                x as i32,
                (y + (14.0 * scale as f32 / 20.0) as u32) as i32,
                pop_scale,
                &font,
                &pop_text
            );
        }
    }
    
    // Draw geographic feature labels
    for label in &map.labels {
        let x = (label.x * scale as f32) as u32;
        let y = (label.y * scale as f32) as u32;
        
        // Choose color based on feature type
        let text_color = match label.feature_type.as_str() {
            "ocean" => Rgb([150, 200, 255]),
            "mountains" => Rgb([150, 150, 150]),
            "forest" => Rgb([100, 200, 100]),
            "swamp" => Rgb([150, 180, 150]),
            "river" => Rgb([100, 150, 255]),
            _ => Rgb([200, 200, 200]),
        };
        
        let label_scale = Scale::uniform(16.0 * (scale as f32 / 20.0));
        
        // Draw black outline for better visibility
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx != 0 || dy != 0 {
                    draw_text_mut(
                        &mut img,
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
        
        // Draw colored text
        draw_text_mut(
            &mut img,
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
            // Place label at midpoint of road
            let mid_idx = road.path.len() / 2;
            let (mid_x, mid_y) = road.path[mid_idx];
            
            let x = (mid_x as u32) * scale;
            let y = (mid_y as u32) * scale;
            
            let road_scale = Scale::uniform(12.0 * (scale as f32 / 20.0));
            
            // Draw black outline
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx != 0 || dy != 0 {
                        draw_text_mut(
                            &mut img,
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
            
            // Draw gray text for roads
            draw_text_mut(
                &mut img,
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
        let x = (bridge.x as u32) * scale;
        let y = (bridge.y as u32) * scale - scale / 2;
        
        let bridge_scale = Scale::uniform(10.0 * (scale as f32 / 20.0));
        
        // Draw black outline
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx != 0 || dy != 0 {
                    draw_text_mut(
                        &mut img,
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
        
        // Draw light blue text for bridges
        draw_text_mut(
            &mut img,
            Rgb([150, 200, 255]),
            x as i32,
            y as i32,
            bridge_scale,
            &font,
            &bridge.name
        );
    }
    
    img.save(filename)?;
    Ok(())
}

fn main() {
    println!("\x1b[1m=== Mapper CLI - Realistic Terrain ===\x1b[0m");
    println!("Procedural Terrain Generator v2.0\n");
    
    loop {
        println!("\n\x1b[1mMenu:\x1b[0m");
        println!("1. Generate new terrain map");
        println!("2. Generate with custom seed");
        println!("3. About");
        println!("4. Exit");
        print!("\nSelect option (1-4): ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => {
                // EOF reached
                println!("\nExiting...");
                break;
            }
            Ok(_) => {},
            Err(e) => {
                println!("Error reading input: {}", e);
                break;
            }
        }
        
        match input.trim() {
            "1" => {
                println!("\n\x1b[1mGenerating terrain map...\x1b[0m\n");
                
                let seed = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as u32;
                
                let mut generator = TerrainGenerator::new(seed);
                let map = generator.generate(80, 30);
                
                print_terrain_ascii(&map);
                print_terrain_info(&map);
                
                // Save high-resolution PNG
                let filename = format!("terrain_map_{}.png", seed);
                match save_terrain_png(&map, &filename, 20) {
                    Ok(_) => println!("\n\x1b[1mHigh-resolution map saved as: \x1b[92m{}\x1b[0m", filename),
                    Err(e) => println!("\n\x1b[91mError saving PNG: {}\x1b[0m", e),
                }
            }
            "2" => {
                print!("Enter seed value (number): ");
                io::stdout().flush().unwrap();
                
                let mut seed_input = String::new();
                io::stdin().read_line(&mut seed_input).unwrap();
                
                match seed_input.trim().parse::<u32>() {
                    Ok(seed) => {
                        println!("\n\x1b[1mGenerating terrain with seed {}...\x1b[0m\n", seed);
                        
                        let mut generator = TerrainGenerator::new(seed);
                        let map = generator.generate(80, 30);
                        
                        print_terrain_ascii(&map);
                        print_terrain_info(&map);
                        
                        // Save high-resolution PNG
                        let filename = format!("terrain_map_{}.png", seed);
                        match save_terrain_png(&map, &filename, 20) {
                            Ok(_) => println!("\n\x1b[1mHigh-resolution map saved as: \x1b[92m{}\x1b[0m", filename),
                            Err(e) => println!("\n\x1b[91mError saving PNG: {}\x1b[0m", e),
                        }
                    }
                    Err(_) => {
                        println!("Invalid seed value. Please enter a number.");
                    }
                }
            }
            "3" => {
                println!("\n\x1b[1mMapper v2.0 - Realistic Terrain Generator\x1b[0m");
                println!("════════════════════════════════════════════");
                println!("Features:");
                println!("  • Perlin noise-based terrain generation");
                println!("  • Multiple biomes based on elevation, moisture, and temperature");
                println!("  • Realistic river generation with water flow");
                println!("  • Procedural place name generation");
                println!("  • Smooth terrain transitions");
                println!("  • \x1b[92mAuto-saves high-resolution PNG images\x1b[0m");
                println!("\nCreated with Rust and the 'noise' crate");
            }
            "4" => {
                println!("Exiting...");
                break;
            }
            "" => {
                // Empty input (just Enter pressed), continue
                continue;
            }
            _ => {
                println!("Invalid option '{}'. Please enter 1-4.", input.trim());
            }
        }
    }
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
        assert!(map.labels.len() > 0, "Should generate some labels");
    }
}
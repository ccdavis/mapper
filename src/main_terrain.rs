mod map_generator;
mod terrain_generator;
mod terrain_renderer;

use terrain_generator::{TerrainGenerator, TerrainMap, Biome, GenerationSettings};
use terrain_renderer::TerrainRenderer;
use std::io::{self, Write};
use std::time::SystemTime;
use std::env;
use image::Rgb;
use rusttype::{Font, Scale};
use imageproc::drawing::draw_text_mut;

fn print_terrain_ascii(map: &TerrainMap) {
    // ASCII representation with sampling for large maps
    let sample_x = (map.width / 80).max(1);
    let sample_y = (map.height / 30).max(1);
    
    for y in (0..map.height).step_by(sample_y) {
        for x in (0..map.width).step_by(sample_x) {
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
                Biome::Mountains => "\x1b[90m",   // Dark gray
                Biome::SnowPeaks => "\x1b[97m",   // White
                Biome::River | Biome::Lake => "\x1b[94m", // Light blue
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
    println!("═══════════════════════════════\n");
    
    // Count biome types
    let mut biome_counts = std::collections::HashMap::new();
    let total_tiles = map.width * map.height;
    
    for row in &map.terrain {
        for point in row {
            *biome_counts.entry(point.biome).or_insert(0) += 1;
        }
    }
    
    println!("\x1b[1mBiome Distribution:\x1b[0m");
    for (biome, count) in biome_counts.iter() {
        let percentage = (*count as f64 / total_tiles as f64) * 100.0;
        println!("  {:?} - {:.1}%", biome, percentage);
    }
    
    println!("\n\x1b[1mRivers:\x1b[0m {} generated", map.rivers.len());
    
    println!("\n\x1b[1mCities:\x1b[0m {} cities", map.cities.len());
    for city in map.cities.iter().take(5) {
        println!("  • {} - Population: {}", city.name, city.population);
    }
    
    println!("\n\x1b[1mRoads:\x1b[0m {} roads", map.roads.len());
    for road in map.roads.iter().take(3) {
        println!("  • {} ({})", road.name, road.road_type);
    }
    
    println!("\n\x1b[1mNamed Locations:\x1b[0m");
    for label in map.labels.iter().take(3) {
        println!("  • {} - {} (at {}, {})", 
                 label.feature_type, label.name, 
                 label.x as usize, label.y as usize);
    }
    
    println!("\n\x1b[1mLegend:\x1b[0m");
    println!("  \x1b[34m≈\x1b[0m Deep Ocean    \x1b[36m~\x1b[0m Ocean       \x1b[96m-\x1b[0m Shore");
    println!("  \x1b[93m.\x1b[0m Beach        \x1b[92m,\x1b[0m Plains      \x1b[32m♣\x1b[0m Forest");
    println!("  \x1b[33mn\x1b[0m Hills        \x1b[90m▲\x1b[0m Mountains   \x1b[97m△\x1b[0m Snow Peaks");
    println!("  \x1b[94m~\x1b[0m Rivers");
}

fn save_terrain_png(map: &TerrainMap, filename: &str, base_scale: u32) -> Result<(), image::ImageError> {
    // Use the shared terrain renderer
    let scale = base_scale; // Direct scale, no multiplication
    let mut img = TerrainRenderer::render_to_image(map, scale);
    
    // Load font for text rendering
    let font_data = include_bytes!("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf");
    let font = Font::try_from_bytes(font_data as &[u8]).unwrap();
    
    // Track occupied label regions to avoid overlaps
    let mut occupied_regions: Vec<(i32, i32, i32, i32)> = Vec::new();
    
    // Helper function to check if a region overlaps with occupied regions
    let check_overlap = |x: i32, y: i32, w: i32, h: i32, occupied: &Vec<(i32, i32, i32, i32)>| -> bool {
        // Add extra margin to prevent labels from being too close
        let margin = 5;
        for &(ox, oy, ow, oh) in occupied {
            if x - margin < ox + ow && x + w + margin > ox && 
               y - margin < oy + oh && y + h + margin > oy {
                return true;
            }
        }
        false
    };
    
    // Helper function to draw a leader line with arrow
    let draw_leader = |img: &mut image::RgbImage, from_x: i32, from_y: i32, to_x: i32, to_y: i32| {
        use imageproc::drawing::{draw_line_segment_mut, draw_filled_circle_mut};
        
        // Draw white outline first for contrast against any background
        let white_color = Rgb([255, 255, 255]);
        let black_color = Rgb([0, 0, 0]);
        
        // Draw thick white outline (5 pixels)
        for offset in -2..=2 {
            draw_line_segment_mut(
                img,
                (from_x as f32 + offset as f32, from_y as f32),
                (to_x as f32 + offset as f32, to_y as f32),
                white_color
            );
            draw_line_segment_mut(
                img,
                (from_x as f32, from_y as f32 + offset as f32),
                (to_x as f32, to_y as f32 + offset as f32),
                white_color
            );
        }
        
        // Draw black center line (3 pixels)
        for offset in -1..=1 {
            draw_line_segment_mut(
                img,
                (from_x as f32 + offset as f32, from_y as f32),
                (to_x as f32 + offset as f32, to_y as f32),
                black_color
            );
            draw_line_segment_mut(
                img,
                (from_x as f32, from_y as f32 + offset as f32),
                (to_x as f32, to_y as f32 + offset as f32),
                black_color
            );
        }
        
        // Draw large arrow head pointing to the city
        let dx = to_x - from_x;
        let dy = to_y - from_y;
        let len = ((dx * dx + dy * dy) as f32).sqrt();
        if len > 10.0 {
            // Normalize direction vector
            let ndx = dx as f32 / len;
            let ndy = dy as f32 / len;
            
            // Larger arrow head
            let arrow_len = 15.0;
            let arrow_width = 8.0;
            
            // Calculate arrow head points - point directly at city
            let arrow_x = to_x as f32;
            let arrow_y = to_y as f32;
            
            // Draw thick arrow head lines
            for offset in -1..=1 {
                draw_line_segment_mut(
                    img,
                    (arrow_x, arrow_y),
                    (arrow_x - ndx * arrow_len - ndy * arrow_width + offset as f32, 
                     arrow_y - ndy * arrow_len + ndx * arrow_width),
                    black_color
                );
                draw_line_segment_mut(
                    img,
                    (arrow_x, arrow_y),
                    (arrow_x - ndx * arrow_len + ndy * arrow_width + offset as f32, 
                     arrow_y - ndy * arrow_len - ndx * arrow_width),
                    black_color
                );
            }
        }
        
        // Draw a larger circle at the arrow point for clarity
        if to_x >= 3 && to_y >= 3 && to_x < img.width() as i32 - 3 && to_y < img.height() as i32 - 3 {
            // White circle with black outline for contrast
            draw_filled_circle_mut(img, (to_x, to_y), 5, Rgb([0, 0, 0]));
            draw_filled_circle_mut(img, (to_x, to_y), 3, Rgb([255, 255, 255]));
        }
    };
    
    // Sort cities by population (draw larger cities first to give them priority)
    let mut sorted_cities: Vec<_> = map.cities.iter().enumerate().collect();
    sorted_cities.sort_by(|a, b| b.1.population.cmp(&a.1.population));
    
    // Draw city labels with smart positioning
    for (_, city) in sorted_cities {
        let city_x = (city.x as u32) * scale + scale / 2;
        let city_y = (city.y as u32) * scale + scale / 2;
        
        // Text scale based on city size - adjusted for readability at any scale
        let text_size_factor = (scale as f32).max(10.0) / 10.0; // Normalize to reasonable text size
        let text_scale = if city.population > 250000 {
            Scale::uniform(28.0 * text_size_factor)  // Large cities
        } else if city.population > 100000 {
            Scale::uniform(24.0 * text_size_factor)  // Medium cities
        } else {
            Scale::uniform(20.0 * text_size_factor)  // Small cities
        };
        
        // Estimate text dimensions with padding for better collision detection
        let char_width = (text_scale.x * 0.6) as i32; // More accurate character width
        let text_width = city.name.len() as i32 * char_width + 10; // Add padding
        let text_height = text_scale.y as i32 + 10; // Add padding
        
        // Try different positions to avoid overlap - more positions for better placement
        let close_offsets = [
            (scale as i32 + 5, -5),  // Right
            (-(text_width + scale as i32 + 5), -5),  // Left
            (-text_width / 2, -(scale as i32 + text_height + 5)),  // Above
            (-text_width / 2, scale as i32 + 5),  // Below
            (scale as i32 + 5, -(scale as i32 + text_height)),  // Right-up
            (-(text_width + scale as i32 + 5), -(scale as i32 + text_height)),  // Left-up
            (scale as i32 + 5, scale as i32),  // Right-down
            (-(text_width + scale as i32 + 5), scale as i32),  // Left-down
        ];
        
        let far_offsets = [
            (scale as i32 * 3, -(scale as i32 * 2)),  // Far right-up
            (-(text_width + scale as i32 * 3), -(scale as i32 * 2)),  // Far left-up
            (scale as i32 * 3, scale as i32 * 2),  // Far right-down
            (-(text_width + scale as i32 * 3), scale as i32 * 2),  // Far left-down
            (scale as i32 * 4, -(scale as i32)),  // Very far right
            (-(text_width + scale as i32 * 4), -(scale as i32)),  // Very far left
            (-text_width / 2, -(scale as i32 * 3 + text_height)),  // Very far above
            (-text_width / 2, scale as i32 * 3),  // Very far below
        ];
        
        let mut best_pos = None;
        let mut needs_leader = false;
        
        // First try close positions without leader lines
        for &(dx, dy) in close_offsets.iter() {
            let test_x = city_x as i32 + dx;
            let test_y = city_y as i32 + dy;
            
            if test_x > 0 && test_y > 0 && 
               test_x + text_width < img.width() as i32 && 
               test_y + text_height < img.height() as i32 &&
               !check_overlap(test_x, test_y, text_width, text_height, &occupied_regions) {
                best_pos = Some((test_x, test_y));
                needs_leader = false;
                break;
            }
        }
        
        // If no close position works, try farther positions with leader lines
        if best_pos.is_none() {
            for &(dx, dy) in far_offsets.iter() {
                let test_x = city_x as i32 + dx;
                let test_y = city_y as i32 + dy;
                
                if test_x > 0 && test_y > 0 && 
                   test_x + text_width < img.width() as i32 && 
                   test_y + text_height < img.height() as i32 &&
                   !check_overlap(test_x, test_y, text_width, text_height, &occupied_regions) {
                    best_pos = Some((test_x, test_y));
                    needs_leader = true;
                    break;
                }
            }
        }
        
        // If still no position found, skip this label to avoid overlap
        let (label_x, label_y) = if let Some(pos) = best_pos {
            pos
        } else {
            // For important cities (large population), try harder to find a spot
            if city.population > 100000 {
                needs_leader = true;
                // Try to find a position with minimal overlap
                let search_radius = scale as i32 * 6;
                let mut best_angle_pos = None;
                let mut min_overlap_count = i32::MAX;
            
                for angle in (0..360).step_by(45) {
                    let rad = (angle as f32) * std::f32::consts::PI / 180.0;
                    let test_x = city_x as i32 + (search_radius as f32 * rad.cos()) as i32 - text_width / 2;
                    let test_y = city_y as i32 + (search_radius as f32 * rad.sin()) as i32 - text_height / 2;
                    
                    if test_x > 0 && test_y > 0 && 
                       test_x + text_width < img.width() as i32 && 
                       test_y + text_height < img.height() as i32 {
                        // Count overlapping labels
                        let mut overlap_count = 0;
                        for &(rx, ry, rw, rh) in occupied_regions.iter() {
                            if test_x < rx + rw && test_x + text_width > rx && 
                               test_y < ry + rh && test_y + text_height > ry {
                                overlap_count += 1;
                            }
                        }
                        
                        if overlap_count < min_overlap_count {
                            min_overlap_count = overlap_count;
                            best_angle_pos = Some((test_x, test_y));
                        }
                        
                        // If we found a spot with no overlaps, use it
                        if overlap_count == 0 {
                            break;
                        }
                    }
                }
                
                // Use the best position found, even if it has some overlap
                // Important cities should always have labels
                best_angle_pos.unwrap_or((city_x as i32 + search_radius, city_y as i32))
            } else {
                // For smaller cities, try progressively farther distances
                let mut found_pos = None;
                for radius_mult in [8, 10, 12, 15].iter() {
                    let search_radius = scale as i32 * radius_mult;
                    for angle in (0..360).step_by(60) {
                        let rad = (angle as f32) * std::f32::consts::PI / 180.0;
                        let test_x = city_x as i32 + (search_radius as f32 * rad.cos()) as i32 - text_width / 2;
                        let test_y = city_y as i32 + (search_radius as f32 * rad.sin()) as i32 - text_height / 2;
                        
                        if test_x > 0 && test_y > 0 && 
                           test_x + text_width < img.width() as i32 && 
                           test_y + text_height < img.height() as i32 &&
                           !check_overlap(test_x, test_y, text_width, text_height, &occupied_regions) {
                            found_pos = Some((test_x, test_y));
                            break;
                        }
                    }
                    if found_pos.is_some() {
                        break;
                    }
                }
                // Always place the label somewhere, even if far away
                found_pos.unwrap_or((city_x as i32 + scale as i32 * 10, city_y as i32 - scale as i32 * 5))
            }
        };
        
        // Draw the label with outline FIRST
        for dy in -2i32..=2 {
            for dx in -2i32..=2 {
                if dx != 0 || dy != 0 {
                    draw_text_mut(
                        &mut img,
                        Rgb([0, 0, 0]),
                        label_x + dx,
                        label_y + dy,
                        text_scale,
                        &font,
                        &city.name
                    );
                }
            }
        }
        
        draw_text_mut(
            &mut img,
            Rgb([255, 255, 255]),
            label_x,
            label_y,
            text_scale,
            &font,
            &city.name
        );
        
        // ALWAYS draw leader line for every city (draw AFTER text so it's visible)
        // Calculate the closest edge of the text box to the city
        let text_center_x = label_x + text_width / 2;
        let text_center_y = label_y + text_height / 2;
        
        // Find the edge point of the text box closest to the city
        let dx = city_x as i32 - text_center_x;
        let dy = city_y as i32 - text_center_y;
        
        let from_x = if dx.abs() > dy.abs() {
            // Connect from left or right edge
            if dx > 0 {
                label_x + text_width  // Right edge
            } else {
                label_x  // Left edge
            }
        } else {
            text_center_x  // Center horizontally
        };
        
        let from_y = if dy.abs() > dx.abs() {
            // Connect from top or bottom edge
            if dy > 0 {
                label_y + text_height  // Bottom edge
            } else {
                label_y  // Top edge
            }
        } else {
            text_center_y  // Center vertically
        };
        
        // Always draw the leader line
        draw_leader(&mut img, from_x, from_y, city_x as i32, city_y as i32);
        
        // Mark this region as occupied
        occupied_regions.push((label_x, label_y, text_width, text_height));
        
        // Draw population if large enough
        if city.population > 100000 {
            let pop_text = format!("({}k)", city.population / 1000);
            let pop_scale = Scale::uniform(16.0 * text_size_factor);
            
            // Position population text below the city name
            let pop_y = label_y + text_height + 5;
            
            draw_text_mut(
                &mut img,
                Rgb([200, 200, 200]),
                label_x,
                pop_y,
                pop_scale,
                &font,
                &pop_text
            );
            
            // Mark population label as occupied too
            let pop_width = (pop_text.len() as i32 * pop_scale.x as i32 * 3) / 5;
            occupied_regions.push((label_x, pop_y, pop_width, pop_scale.y as i32));
        }
    }
    
    // Draw road labels (only for major roads to avoid clutter)
    for road in &map.roads {
        if road.road_type == "highway" && road.path.len() > 10 {
            // Draw label at midpoint of road
            let mid_idx = road.path.len() / 2;
            let (rx, ry) = road.path[mid_idx];
            let x = (rx as u32) * scale;
            let y = (ry as u32) * scale;
            
            let text_size_factor = (scale as f32).max(10.0) / 10.0;
            let road_scale = Scale::uniform(16.0 * text_size_factor);
            
            // Draw with outline for visibility
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx != 0 || dy != 0 {
                        draw_text_mut(
                            &mut img,
                            Rgb([255, 255, 255]),
                            x as i32 + dx,
                            y as i32 + dy,
                            road_scale,
                            &font,
                            &road.name
                        );
                    }
                }
            }
            
            draw_text_mut(
                &mut img,
                Rgb([60, 60, 60]),
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
        
        let text_size_factor = (scale as f32).max(10.0) / 10.0;
        let bridge_scale = Scale::uniform(14.0 * text_size_factor);
        
        // Draw with white outline
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx != 0 || dy != 0 {
                    draw_text_mut(
                        &mut img,
                        Rgb([255, 255, 255]),
                        x as i32 + dx,
                        y as i32 + dy,
                        bridge_scale,
                        &font,
                        &bridge.name
                    );
                }
            }
        }
        
        draw_text_mut(
            &mut img,
            Rgb([80, 60, 40]),
            x as i32,
            y as i32,
            bridge_scale,
            &font,
            &bridge.name
        );
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
        
        // Much larger font sizes for geographic features
        let text_size_factor = (scale as f32).max(10.0) / 10.0;
        let label_scale = match label.feature_type.as_str() {
            "ocean" => Scale::uniform(32.0 * text_size_factor),     // Oceans - large
            "mountains" => Scale::uniform(26.0 * text_size_factor), // Mountains - medium-large
            "forest" => Scale::uniform(22.0 * text_size_factor),    // Forests - medium
            "swamp" => Scale::uniform(22.0 * text_size_factor),     // Swamps - medium
            "river" => Scale::uniform(18.0 * text_size_factor),     // Rivers - small-medium
            _ => Scale::uniform(20.0 * text_size_factor),           // Default
        };
        
        // Draw black outline for better visibility
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx != 0 || dy != 0 {
                    draw_text_mut(
                        &mut img,
                        Rgb([0, 0, 0]),
                        x as i32 + dx,
                        y as i32 + dy,
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
    
    img.save(filename)?;
    Ok(())
}

fn parse_args() -> GenerationSettings {
    let args: Vec<String> = env::args().collect();
    let mut settings = GenerationSettings::default();
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--rivers" => {
                if i + 1 < args.len() {
                    if let Ok(value) = args[i + 1].parse::<f32>() {
                        settings.river_density = value.clamp(0.0, 1.0);
                        i += 1;
                    }
                }
            }
            "--cities" => {
                if i + 1 < args.len() {
                    if let Ok(value) = args[i + 1].parse::<f32>() {
                        settings.city_density = value.clamp(0.0, 1.0);
                        i += 1;
                    }
                }
            }
            "--land" => {
                if i + 1 < args.len() {
                    if let Ok(value) = args[i + 1].parse::<f32>() {
                        settings.land_percentage = value.clamp(0.0, 1.0);
                        i += 1;
                    }
                }
            }
            "--help" => {
                println!("Terrain Generator CLI");
                println!("\nUsage: mapper-terrain-cli [OPTIONS]");
                println!("\nOptions:");
                println!("  --rivers <0.0-1.0>  Set river density (default: 0.5)");
                println!("  --cities <0.0-1.0>  Set city density (default: 0.5)");
                println!("  --land <0.0-1.0>    Set land percentage (default: 0.4)");
                println!("  --help              Show this help message");
                println!("\nExample:");
                println!("  mapper-terrain-cli --rivers 0.8 --cities 0.3 --land 0.6");
                std::process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }
    
    settings
}

fn main() {
    let settings = parse_args();
    
    // Check if we should run in quick mode (if any settings were provided via CLI)
    let args: Vec<String> = std::env::args().collect();
    let quick_mode = args.len() > 1 && args.iter().any(|arg| 
        arg.starts_with("--land") || arg.starts_with("--rivers") || arg.starts_with("--cities"));
    
    if quick_mode {
        // Quick mode: generate immediately and exit
        println!("Generating terrain map with settings: Rivers={:.0}%, Cities={:.0}%, Land={:.0}%",
                 settings.river_density * 100.0,
                 settings.city_density * 100.0,
                 settings.land_percentage * 100.0);
        
        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32;
        let mut generator = TerrainGenerator::new_with_settings(seed, settings);
        let map = generator.generate(320, 240);  // Ultra-high resolution: 320x240 tiles
        
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let filename = format!("terrain_map_{}.png", timestamp);
        
        match save_terrain_png(&map, &filename, 5) {
            Ok(_) => println!("Map saved as: {}", filename),
            Err(e) => eprintln!("Error saving map: {}", e),
        }
        return;
    }
    
    loop {
        println!("\n\x1b[1mMenu:\x1b[0m");
        println!("1. Generate new terrain map");
        println!("2. Generate with custom seed");
        println!("3. About");
        println!("4. Exit");
        println!("\nCurrent settings: Rivers={:.0}%, Cities={:.0}%, Land={:.0}%",
                 settings.river_density * 100.0,
                 settings.city_density * 100.0,
                 settings.land_percentage * 100.0);
        
        print!("\nSelect option (1-4): ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let choice = input.trim();
        
        match choice {
            "1" => {
                let seed = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as u32;
                let mut generator = TerrainGenerator::new_with_settings(seed, settings);
                let map = generator.generate(320, 240);  // Ultra-high resolution
                
                println!("\n\x1b[1mGenerated Terrain Map:\x1b[0m\n");
                print_terrain_ascii(&map);
                print_terrain_info(&map);
                
                let timestamp = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let filename = format!("terrain_map_{}.png", timestamp);
                
                match save_terrain_png(&map, &filename, 5) {
                    Ok(_) => println!("\n\x1b[1mHigh-resolution map saved as: \x1b[92m{}\x1b[0m", filename),
                    Err(e) => eprintln!("\x1b[91mError saving map: {}\x1b[0m", e),
                }
            },
            "2" => {
                print!("Enter seed value: ");
                io::stdout().flush().unwrap();
                
                let mut seed_input = String::new();
                io::stdin().read_line(&mut seed_input).unwrap();
                
                match seed_input.trim().parse::<u32>() {
                    Ok(seed) => {
                        let mut generator = TerrainGenerator::new_with_settings(seed, settings);
                        let map = generator.generate(320, 240);  // Ultra-high resolution
                        
                        println!("\n\x1b[1mGenerated Terrain Map (Seed: {}):\x1b[0m\n", seed);
                        print_terrain_ascii(&map);
                        print_terrain_info(&map);
                        
                        let filename = format!("terrain_map_seed_{}.png", seed);
                        
                        match save_terrain_png(&map, &filename, 5) {
                            Ok(_) => println!("\n\x1b[1mHigh-resolution map saved as: \x1b[92m{}\x1b[0m", filename),
                            Err(e) => eprintln!("\x1b[91mError saving map: {}\x1b[0m", e),
                        }
                    },
                    Err(_) => println!("\x1b[91mInvalid seed value. Please enter a number.\x1b[0m"),
                }
            },
            "3" => {
                println!("\n\x1b[1mTerrain Generator\x1b[0m");
                println!("═══════════════════");
                println!("A procedural terrain generation system that creates");
                println!("realistic landscapes with:");
                println!("  • Varied biomes (oceans, mountains, forests, etc.)");
                println!("  • Rivers flowing from mountains to seas");
                println!("  • Cities with Zipf's law population distribution");
                println!("  • Roads connecting cities with pathfinding");
                println!("  • Geographic feature names");
                println!("\nMaps are saved as high-resolution PNG images.");
            },
            "4" => {
                println!("\nExiting...");
                break;
            },
            _ => {
                println!("\x1b[91mInvalid option. Please select 1-4.\x1b[0m");
            }
        }
    }
}
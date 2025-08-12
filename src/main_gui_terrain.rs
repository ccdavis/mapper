mod map_generator;
mod terrain_generator;
mod terrain_renderer;

use terrain_generator::{TerrainGenerator, TerrainMap, GenerationSettings};
use terrain_renderer::TerrainRenderer;
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};
use std::time::SystemTime;
use rusttype::{Font, Scale};
use imageproc::drawing::draw_text_mut;
use image::{ImageBuffer, Rgb};
use std::thread;

slint::include_modules!();

fn generate_terrain_image(map: &TerrainMap) -> Image {
    let width = map.width;
    let height = map.height;
    let scale = 2; // Tiny tiles - each tile is only 2x2 pixels for maximum map visibility
    
    // Use the shared terrain renderer
    let pixels = TerrainRenderer::render_to_pixels(map, width, height, scale);
    
    let img_width = width * scale;
    let img_height = height * scale;
    
    // Convert to Slint's pixel buffer format
    let mut pixel_buffer = SharedPixelBuffer::<Rgba8Pixel>::new(img_width as u32, img_height as u32);
    let dest_pixels = pixel_buffer.make_mut_bytes();
    
    // Copy pixels from renderer output to Slint buffer
    for i in 0..pixels.len() {
        dest_pixels[i] = pixels[i];
    }
    
    // Draw text labels using image crate, then convert back
    let mut img: image::RgbaImage = ImageBuffer::from_raw(
        img_width as u32, 
        img_height as u32, 
        pixels.clone()
    ).unwrap();
    
    // Load font for text rendering
    if let Ok(font_data) = std::fs::read("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf") {
        if let Some(font) = Font::try_from_vec(font_data) {
            // Convert to RGB for text rendering
            let mut rgb_img = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
            
            // Track occupied regions to avoid overlap
            let mut occupied_regions: Vec<(i32, i32, i32, i32)> = Vec::new();
            
            // Helper function to check overlap
            let check_overlap = |x: i32, y: i32, w: i32, h: i32, regions: &Vec<(i32, i32, i32, i32)>| -> bool {
                for &(rx, ry, rw, rh) in regions {
                    if x < rx + rw && x + w > rx && y < ry + rh && y + h > ry {
                        return true;
                    }
                }
                false
            };
            
            // Sort cities by population (draw larger cities first)
            let mut sorted_cities: Vec<_> = map.cities.iter().collect();
            sorted_cities.sort_by(|a, b| b.population.cmp(&a.population));
            
            // Draw city labels with smart positioning
            for city in sorted_cities {
                let city_x = city.x * scale + scale / 2;
                let city_y = city.y * scale + scale / 2;
                
                // Text scale for GUI's 2x scale
                let text_scale = if city.population > 250000 {
                    Scale::uniform(11.0)
                } else if city.population > 100000 {
                    Scale::uniform(10.0)
                } else {
                    Scale::uniform(9.0)
                };
                
                // Estimate text dimensions
                let text_width = (city.name.len() as i32 * text_scale.x as i32 * 3) / 5;
                let text_height = text_scale.y as i32;
                
                // Try different positions to avoid overlap
                let offsets = [
                    (scale as i32, -(scale as i32) / 2),  // Right of city
                    (-(text_width + scale as i32), -(scale as i32) / 2),  // Left
                    (scale as i32 / 2 - text_width / 2, -(scale as i32 + text_height)),  // Above
                    (scale as i32 / 2 - text_width / 2, scale as i32),  // Below
                ];
                
                let mut best_pos = None;
                for &(dx, dy) in offsets.iter() {
                    let test_x = city_x as i32 + dx;
                    let test_y = city_y as i32 + dy;
                    
                    if !check_overlap(test_x, test_y, text_width, text_height, &occupied_regions) {
                        best_pos = Some((test_x, test_y));
                        break;
                    }
                }
                
                let (label_x, label_y) = best_pos.unwrap_or((city_x as i32 + scale as i32, city_y as i32));
                
                // Draw outline for better visibility
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx != 0 || dy != 0 {
                            draw_text_mut(
                                &mut rgb_img,
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
                
                // Draw city name
                draw_text_mut(
                    &mut rgb_img,
                    Rgb([255, 255, 255]),
                    label_x,
                    label_y,
                    text_scale,
                    &font,
                    &city.name
                );
                
                occupied_regions.push((label_x, label_y, text_width, text_height));
                
                // Draw population for large cities
                if city.population > 100000 {
                    let pop_text = format!("({}k)", city.population / 1000);
                    let pop_scale = Scale::uniform(8.0);
                    let pop_y = label_y + text_height + 2;
                    
                    draw_text_mut(
                        &mut rgb_img,
                        Rgb([200, 200, 200]),
                        label_x,
                        pop_y,
                        pop_scale,
                        &font,
                        &pop_text
                    );
                }
            }
            
            // Draw geographic labels
            for label in &map.labels {
                let x = (label.x * scale as f32) as i32;
                let y = (label.y * scale as f32) as i32;
                
                let text_color = match label.feature_type.as_str() {
                    "ocean" => Rgb([150, 200, 255]),
                    "mountains" => Rgb([150, 150, 150]),
                    "forest" => Rgb([100, 200, 100]),
                    "swamp" => Rgb([150, 180, 150]),
                    "river" => Rgb([100, 150, 255]),
                    _ => Rgb([200, 200, 200]),
                };
                
                let label_scale = match label.feature_type.as_str() {
                    "ocean" => Scale::uniform(14.0),
                    "mountains" => Scale::uniform(12.0),
                    "forest" => Scale::uniform(11.0),
                    "swamp" => Scale::uniform(11.0),
                    "river" => Scale::uniform(10.0),
                    _ => Scale::uniform(11.0),
                };
                
                // Draw outline
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx != 0 || dy != 0 {
                            draw_text_mut(
                                &mut rgb_img,
                                Rgb([0, 0, 0]),
                                x + dx,
                                y + dy,
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
                    x,
                    y,
                    label_scale,
                    &font,
                    &label.name
                );
            }
            
            // Convert back to RGBA
            img = image::DynamicImage::ImageRgb8(rgb_img).to_rgba8();
        }
    }
    
    // Copy the final image back to the pixel buffer
    let final_pixels = img.as_raw();
    let dest_pixels = pixel_buffer.make_mut_bytes();
    for i in 0..final_pixels.len() {
        dest_pixels[i] = final_pixels[i];
    }
    
    Image::from_rgba8(pixel_buffer)
}

fn generate_map_info(map: &TerrainMap) -> String {
    let mut info = String::new();
    
    // Count biome types
    let mut biome_counts = std::collections::HashMap::new();
    let total_tiles = map.width * map.height;
    
    for row in &map.terrain {
        for point in row {
            *biome_counts.entry(point.biome).or_insert(0) += 1;
        }
    }
    
    info.push_str("Biome Distribution:\n");
    for (biome, count) in biome_counts.iter() {
        let percentage = (*count as f64 / total_tiles as f64) * 100.0;
        info.push_str(&format!("  {:?} - {:.1}%\n", biome, percentage));
    }
    
    info.push_str(&format!("\nRivers: {} generated\n", map.rivers.len()));
    info.push_str(&format!("Cities: {} cities\n", map.cities.len()));
    
    for city in map.cities.iter().take(5) {
        info.push_str(&format!("  • {} - Pop: {}\n", city.name, city.population));
    }
    
    info.push_str(&format!("\nRoads: {} roads\n", map.roads.len()));
    for road in map.roads.iter().take(3) {
        info.push_str(&format!("  • {}\n", road.name));
    }
    
    info
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = MapperWindow::new()?;
    
    let ui_handle = ui.as_weak();
    ui.on_menu_start(move || {
        let ui = ui_handle.unwrap();
        
        // Get settings from UI before spawning thread
        let settings = GenerationSettings {
            river_density: ui.get_river_density(),
            city_density: ui.get_city_density(),
            land_percentage: ui.get_land_percentage(),
        };
        
        // Clone the weak handle for use in the thread
        let ui_handle_thread = ui_handle.clone();
        
        // Generate map in a separate thread to keep UI responsive
        thread::spawn(move || {
            // Generate terrain with current timestamp as seed
            let seed = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as u32;
            
            let mut generator = TerrainGenerator::new_with_settings(seed, settings);
            
            // Generate a huge map - 1600x1000 tiles
            let map = generator.generate(1600, 1000);
            let info = generate_map_info(&map);
            
            // Update UI from main thread
            let _ = slint::invoke_from_event_loop(move || {
                let ui = ui_handle_thread.unwrap();
                let image = generate_terrain_image(&map);
                ui.set_map_image(image);
                ui.set_map_status(format!("Map generated (Seed: {})\n{}", seed, info).into());
                ui.set_has_map(true);
                ui.set_is_generating(false);
            });
        });
    });
    
    ui.on_menu_exit(move || {
        std::process::exit(0);
    });
    
    ui.on_menu_about(move || {
        // About is handled in the UI
    });
    
    ui.run()
}
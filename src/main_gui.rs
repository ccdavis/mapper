mod map_generator;

use map_generator::{Map, TileType};
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

slint::include_modules!();

fn generate_map_image(map: &Map) -> Image {
    let width = map.width * 20;
    let height = map.height * 20;
    let mut pixel_buffer = SharedPixelBuffer::<Rgba8Pixel>::new(width as u32, height as u32);
    
    let pixels = pixel_buffer.make_mut_bytes();
    
    for y in 0..map.height {
        for x in 0..map.width {
            let color = match map.tiles[y][x] {
                TileType::Water => [30, 144, 255, 255],   // Blue
                TileType::Grass => [34, 139, 34, 255],    // Green
                TileType::Dirt => [139, 115, 85, 255],    // Brown
                TileType::Stone => [128, 128, 128, 255],  // Gray
                TileType::Sand => [238, 203, 173, 255],   // Sandy
            };
            
            // Fill a 20x20 pixel tile
            for ty in 0..20 {
                for tx in 0..20 {
                    let px = x * 20 + tx;
                    let py = y * 20 + ty;
                    let pixel_index = ((py * width + px) * 4) as usize;
                    
                    if pixel_index + 3 < pixels.len() {
                        pixels[pixel_index] = color[0];
                        pixels[pixel_index + 1] = color[1];
                        pixels[pixel_index + 2] = color[2];
                        pixels[pixel_index + 3] = color[3];
                    }
                }
            }
        }
    }
    
    Image::from_rgba8(pixel_buffer)
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = MapperWindow::new()?;
    
    let ui_handle = ui.as_weak();
    ui.on_menu_start(move || {
        let ui = ui_handle.unwrap();
        
        // Generate a new map
        let mut map = Map::new(40, 30);
        map.generate_random();
        
        // Convert to image
        let map_image = generate_map_image(&map);
        
        // Update UI
        ui.set_map_image(map_image);
        ui.set_has_map(true);
        ui.set_map_status("Map generated successfully".into());
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
        // In Slint, we handle the about dialog in the .slint file
        // For now, just update status
        ui.set_map_status("Mapper v0.1.0 - A procedural map generation tool".into());
    });
    
    ui.run()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_map_image_generation() {
        let mut map = Map::new(10, 10);
        map.generate_random();
        let image = generate_map_image(&map);
        
        // Check that image is created (basic validation)
        // Slint Image doesn't expose dimensions directly in tests
        // but we can verify it was created without panic
        assert_eq!(format!("{:?}", image).is_empty(), false);
    }
}
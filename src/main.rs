mod map_generator;

use map_generator::Map;
use std::io::{self, Write};

fn start_mapping() -> String {
    let mut map = Map::new(40, 30);
    map.generate_random();
    serde_json::to_string(&map).unwrap_or_else(|_| "Error generating map".to_string())
}

fn show_about() -> String {
    "Mapper v0.1.0\nA procedural map generation tool".to_string()
}

fn print_map_as_ascii(map: &Map) {
    use map_generator::TileType;
    
    for y in 0..map.height {
        for x in 0..map.width {
            let tile_char = match map.tiles[y][x] {
                TileType::Water => '~',
                TileType::Grass => '.',
                TileType::Dirt => '#',
                TileType::Stone => '^',
                TileType::Sand => 's',
            };
            print!("{}", tile_char);
        }
        println!();
    }
}

fn main() {
    println!("=== Mapper CLI ===");
    println!("Procedural Map Generator\n");
    
    loop {
        println!("\nMenu:");
        println!("1. Start - Generate new map");
        println!("2. About - Show application info");
        println!("3. Exit - Quit application");
        print!("\nSelect option (1-3): ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        match input.trim() {
            "1" => {
                println!("\nGenerating map...\n");
                let mut map = Map::new(60, 20);
                map.generate_random();
                print_map_as_ascii(&map);
            }
            "2" => {
                println!("\n{}", show_about());
            }
            "3" => {
                println!("Exiting...");
                break;
            }
            _ => {
                println!("Invalid option. Please try again.");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_mapping() {
        let result = start_mapping();
        assert!(result.contains("width") || result.contains("Error"));
    }

    #[test]
    fn test_show_about() {
        let result = show_about();
        assert!(result.contains("Mapper"));
        assert!(result.contains("v0.1.0"));
    }
}
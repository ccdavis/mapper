use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TileType {
    Water,
    Grass,
    Dirt,
    Stone,
    Sand,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Map {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<Vec<TileType>>,
}

impl Map {
    pub fn new(width: usize, height: usize) -> Self {
        let tiles = vec![vec![TileType::Grass; width]; height];
        Map { width, height, tiles }
    }
    
    pub fn generate_random(&mut self) {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hash, Hasher};
        
        let random_state = RandomState::new();
        
        for y in 0..self.height {
            for x in 0..self.width {
                let mut hasher = random_state.build_hasher();
                (x, y, std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()).hash(&mut hasher);
                let hash = hasher.finish();
                let value = (hash % 100) as f32 / 100.0;
                
                self.tiles[y][x] = if value < 0.2 {
                    TileType::Water
                } else if value < 0.5 {
                    TileType::Grass
                } else if value < 0.7 {
                    TileType::Dirt
                } else if value < 0.85 {
                    TileType::Stone
                } else {
                    TileType::Sand
                };
            }
        }
    }
    
    pub fn get_tile(&self, x: usize, y: usize) -> Option<TileType> {
        if x < self.width && y < self.height {
            Some(self.tiles[y][x])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_map_creation() {
        let map = Map::new(10, 10);
        assert_eq!(map.width, 10);
        assert_eq!(map.height, 10);
        assert_eq!(map.tiles.len(), 10);
        assert_eq!(map.tiles[0].len(), 10);
    }
    
    #[test]
    fn test_get_tile() {
        let map = Map::new(5, 5);
        assert_eq!(map.get_tile(0, 0), Some(TileType::Grass));
        assert_eq!(map.get_tile(4, 4), Some(TileType::Grass));
        assert_eq!(map.get_tile(5, 5), None);
    }
    
    #[test]
    fn test_generate_random() {
        let mut map = Map::new(20, 20);
        map.generate_random();
        
        let mut has_different_tiles = false;
        let first_tile = map.tiles[0][0];
        
        for row in &map.tiles {
            for &tile in row {
                if tile != first_tile {
                    has_different_tiles = true;
                    break;
                }
            }
        }
        
        assert!(has_different_tiles, "Generated map should have varied tile types");
    }
}
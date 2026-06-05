use std::collections::VecDeque;

use noise::NoiseFn;

use super::TerrainGenerator;

impl TerrainGenerator {
    /// Generate the moisture field: a blend of noise and proximity to the
    /// ocean, so coasts are wet, interiors are dry, and deserts/swamps land
    /// in places that make geographic sense.
    pub(super) fn generate_moisture_field(&self, elevations: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let height = elevations.len();
        let width = elevations[0].len();

        // Multi-source BFS distance (in tiles) from the nearest water tile
        let mut dist = vec![vec![u32::MAX; width]; height];
        let mut queue = VecDeque::new();
        for y in 0..height {
            for x in 0..width {
                if elevations[y][x] < 0.0 {
                    dist[y][x] = 0;
                    queue.push_back((x, y));
                }
            }
        }
        while let Some((x, y)) = queue.pop_front() {
            let d = dist[y][x];
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                        continue;
                    }
                    let (nx, ny) = (nx as usize, ny as usize);
                    if dist[ny][nx] == u32::MAX {
                        dist[ny][nx] = d + 1;
                        queue.push_back((nx, ny));
                    }
                }
            }
        }

        // Moisture decays inland over roughly this many tiles
        let decay = (width.min(height) as f64 / 12.0).max(4.0);

        let scale = 1.0 / width.min(height) as f64;
        let mut moisture = vec![vec![0.0; width]; height];
        for y in 0..height {
            for x in 0..width {
                let nx = x as f64 * scale;
                let ny = y as f64 * scale;
                let noise01 = self.moisture_noise.get([nx * 3.0, ny * 3.0]) * 0.5 + 0.5;
                let ocean = (-(dist[y][x] as f64) / decay).exp();
                moisture[y][x] = (noise01 * 0.55 + ocean * 0.45).clamp(0.0, 1.0);
            }
        }

        moisture
    }

    pub(super) fn generate_temperature(
        &self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        elevation: f64,
    ) -> f64 {
        let scale = 1.0 / width.min(height) as f64;
        let nx = x as f64 * scale;
        let ny = y as f64 * scale;

        // Temperature decreases with elevation and latitude
        let base_temp = self.temperature_noise.get([nx * 2.0, ny * 2.0]) * 0.5 + 0.5;
        let latitude_factor = (y as f64 / height as f64 - 0.5).abs() * 2.0;
        let elevation_factor = (elevation + 1.0) / 2.0;

        let temperature =
            base_temp * (1.0 - latitude_factor * 0.3) * (1.0 - elevation_factor * 0.4);
        temperature.clamp(0.0, 1.0)
    }
}

//! River and lake generation.
//!
//! Uses the standard priority-flood algorithm: depressions in the elevation
//! field are filled to their spill level, which guarantees every land tile
//! has a monotone downhill path to the ocean (or map edge). Filled
//! depressions become lakes; rain is then accumulated down the flow
//! directions, and tiles whose drainage area exceeds a density-controlled
//! threshold become rivers. Rivers therefore always reach the sea, join at
//! confluences, and widen downstream.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use super::biome::Biome;
use super::types::TerrainPoint;
use super::TerrainGenerator;

/// Min-heap node for the priority flood.
struct FloodNode {
    elev: f64,
    idx: usize,
}

impl PartialEq for FloodNode {
    fn eq(&self, other: &Self) -> bool {
        self.elev == other.elev
    }
}
impl Eq for FloodNode {}
impl Ord for FloodNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reversed: BinaryHeap is a max-heap, we want the lowest elevation
        other.elev.partial_cmp(&self.elev).unwrap_or(Ordering::Equal)
    }
}
impl PartialOrd for FloodNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

const NEIGHBORS: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

impl TerrainGenerator {
    /// Generate rivers and lakes. Marks lake tiles in `terrain` directly and
    /// returns the river polylines (each traced from source to mouth).
    pub(super) fn generate_hydrology(
        &mut self,
        terrain: &mut [Vec<TerrainPoint>],
    ) -> Vec<Vec<(usize, usize)>> {
        if self.settings.river_density < 0.01 {
            return Vec::new();
        }

        let height = terrain.len();
        let width = terrain[0].len();
        let n = width * height;
        let idx_of = |x: usize, y: usize| y * width + x;

        let elev: Vec<f64> = terrain
            .iter()
            .flat_map(|row| row.iter().map(|p| p.elevation))
            .collect();

        // --- Priority flood: fill depressions to their spill level ---
        let mut filled = elev.clone();
        let mut visited = vec![false; n];
        let mut heap = BinaryHeap::new();

        // Seeds: every water tile, plus land tiles on the map border (they
        // drain off-map).
        for y in 0..height {
            for x in 0..width {
                let i = idx_of(x, y);
                if elev[i] < 0.0 || x == 0 || y == 0 || x == width - 1 || y == height - 1 {
                    visited[i] = true;
                    heap.push(FloodNode { elev: filled[i], idx: i });
                }
            }
        }

        while let Some(FloodNode { elev: cur_elev, idx }) = heap.pop() {
            let x = (idx % width) as i32;
            let y = (idx / width) as i32;
            for (dx, dy) in NEIGHBORS {
                let nx = x + dx;
                let ny = y + dy;
                if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                    continue;
                }
                let ni = idx_of(nx as usize, ny as usize);
                if visited[ni] {
                    continue;
                }
                visited[ni] = true;
                // A tile inside a depression is raised to just above the
                // lowest spill point seen so far.
                filled[ni] = elev[ni].max(cur_elev + 1e-6);
                heap.push(FloodNode { elev: filled[ni], idx: ni });
            }
        }

        // --- Lakes: tiles raised by the fill are under a lake surface ---
        let mut is_lake = vec![false; n];
        for i in 0..n {
            // Only depressions of real depth become lakes (elevations are
            // area quantiles, so 0.004 is a meaningful basin, not noise)
            if elev[i] >= 0.0 && filled[i] > elev[i] + 0.004 {
                is_lake[i] = true;
            }
        }
        // Drop tiny one/two-tile puddles: keep lakes with >= 4 connected tiles
        let mut lake_kept = vec![false; n];
        let mut seen = vec![false; n];
        for start in 0..n {
            if is_lake[start] && !seen[start] {
                let mut component = vec![start];
                let mut stack = vec![start];
                seen[start] = true;
                while let Some(i) = stack.pop() {
                    let x = (i % width) as i32;
                    let y = (i / width) as i32;
                    for (dx, dy) in NEIGHBORS {
                        let nx = x + dx;
                        let ny = y + dy;
                        if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                            continue;
                        }
                        let ni = idx_of(nx as usize, ny as usize);
                        if is_lake[ni] && !seen[ni] {
                            seen[ni] = true;
                            stack.push(ni);
                            component.push(ni);
                        }
                    }
                }
                if component.len() >= 4 {
                    for i in component {
                        lake_kept[i] = true;
                    }
                }
            }
        }
        for i in 0..n {
            if lake_kept[i] {
                terrain[i / width][i % width].biome = Biome::Lake;
            }
        }

        // --- Flow directions: steepest descent on the filled surface ---
        let mut downstream = vec![usize::MAX; n];
        for y in 0..height {
            for x in 0..width {
                let i = idx_of(x, y);
                let mut best = filled[i];
                for (dx, dy) in NEIGHBORS {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                        continue;
                    }
                    let ni = idx_of(nx as usize, ny as usize);
                    if filled[ni] < best {
                        best = filled[ni];
                        downstream[i] = ni;
                    }
                }
            }
        }

        // --- Flow accumulation: rain one unit on every tile, pour downhill ---
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| filled[b].partial_cmp(&filled[a]).unwrap_or(Ordering::Equal));
        let mut acc = vec![1.0f64; n];
        for &i in &order {
            let ds = downstream[i];
            if ds != usize::MAX {
                acc[ds] += acc[i];
            }
        }

        // --- Threshold drainage area by river density ---
        let land_tiles = elev.iter().filter(|&&e| e >= 0.0).count().max(1);
        let density = self.settings.river_density as f64;
        let threshold = ((land_tiles as f64 / 200.0) * (2.2 - 2.0 * density)).max(20.0);

        let mut is_river = vec![false; n];
        for i in 0..n {
            if elev[i] >= 0.0 && acc[i] >= threshold {
                is_river[i] = true;
            }
        }

        // --- Trace polylines from each river head downstream ---
        // A head is a river tile with no river tile flowing into it.
        let mut has_river_upstream = vec![false; n];
        for i in 0..n {
            if is_river[i] {
                let ds = downstream[i];
                if ds != usize::MAX {
                    has_river_upstream[ds] = true;
                }
            }
        }

        let mut rivers = Vec::new();
        let mut claimed = vec![false; n];
        for head in 0..n {
            if !is_river[head] || has_river_upstream[head] {
                continue;
            }
            let mut path = Vec::new();
            let mut i = head;
            loop {
                path.push((i % width, i / width));
                if claimed[i] {
                    // Joined an already-traced river: stop at the confluence
                    break;
                }
                claimed[i] = true;
                if elev[i] < 0.0 {
                    // Reached the sea
                    break;
                }
                let ds = downstream[i];
                if ds == usize::MAX {
                    // Drains off the map edge
                    break;
                }
                i = ds;
            }
            if path.len() >= 6 {
                rivers.push(path);
            }
        }

        rivers
    }
}

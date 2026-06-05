use super::biome::Biome;
use super::types::{PlaceLabel, TerrainPoint};
use super::TerrainGenerator;

/// Which name generator to use for a labeled region.
#[derive(Copy, Clone)]
enum RegionKind {
    Ocean,
    Mountains,
    Forest,
    Swamp,
}

/// Configuration for one class of labeled region: biome predicate, how many
/// labels at most, and the minimum region size worth naming.
struct RegionLabelSpec {
    kind: RegionKind,
    feature_type: &'static str,
    predicate: fn(&Biome) -> bool,
    max_labels: usize,
    min_size: usize,
}

const REGION_SPECS: [RegionLabelSpec; 4] = [
    RegionLabelSpec {
        kind: RegionKind::Ocean,
        feature_type: "ocean",
        predicate: |b| matches!(b, Biome::Ocean | Biome::DeepOcean),
        max_labels: 3,
        min_size: 200,
    },
    RegionLabelSpec {
        kind: RegionKind::Mountains,
        feature_type: "mountains",
        predicate: |b| matches!(b, Biome::Mountains | Biome::SnowPeaks),
        max_labels: 4,
        min_size: 40,
    },
    RegionLabelSpec {
        kind: RegionKind::Forest,
        feature_type: "forest",
        predicate: |b| matches!(b, Biome::Forest),
        max_labels: 3,
        min_size: 100,
    },
    RegionLabelSpec {
        kind: RegionKind::Swamp,
        feature_type: "swamp",
        predicate: |b| matches!(b, Biome::Swamp),
        max_labels: 2,
        min_size: 60,
    },
];

impl TerrainGenerator {
    pub(super) fn generate_labels(
        &mut self,
        terrain: &[Vec<TerrainPoint>],
        rivers: &[Vec<(usize, usize)>],
    ) -> Vec<PlaceLabel> {
        let mut labels = Vec::new();
        let mut placed_labels: Vec<(f32, f32)> = Vec::new();

        // Scale minimum distance between labels based on map size
        let map_scale = (terrain[0].len() as f32 / 160.0).max(terrain.len() as f32 / 120.0);
        let min_distance = 80.0 * map_scale;
        let is_too_close = |x: f32, y: f32, placed: &Vec<(f32, f32)>| -> bool {
            placed
                .iter()
                .any(|&(px, py)| ((x - px).powi(2) + (y - py).powi(2)).sqrt() < min_distance)
        };

        // Label the largest regions of each kind
        for spec in &REGION_SPECS {
            let mut regions = self.find_regions(terrain, spec.predicate);
            regions.sort_by(|a, b| b.len().cmp(&a.len()));

            for (i, region) in regions.iter().take(spec.max_labels).enumerate() {
                if region.len() <= spec.min_size {
                    continue;
                }
                let (cx, cy) = self.region_center(region);
                let fx = cx as f32;
                let fy = cy as f32;
                if is_too_close(fx, fy, &placed_labels) {
                    continue;
                }
                let name = match spec.kind {
                    RegionKind::Ocean => self.generate_ocean_name(i),
                    RegionKind::Mountains => self.generate_mountain_name(i),
                    RegionKind::Forest => self.generate_forest_name(i),
                    RegionKind::Swamp => self.generate_swamp_name(i),
                };
                labels.push(PlaceLabel {
                    x: fx,
                    y: fy,
                    name,
                    feature_type: spec.feature_type.to_string(),
                });
                placed_labels.push((fx, fy));
            }
        }

        // River names - only major rivers, well-spaced
        let mut river_labels_added = 0;
        for (i, river) in rivers.iter().enumerate() {
            if river.len() > 30 && river_labels_added < 3 {
                // Place label at a good position along the river
                let positions = [river.len() / 3, river.len() / 2, river.len() * 2 / 3];
                for pos in positions {
                    if pos < river.len() {
                        let fx = river[pos].0 as f32;
                        let fy = river[pos].1 as f32;
                        if !is_too_close(fx, fy, &placed_labels) {
                            labels.push(PlaceLabel {
                                x: fx,
                                y: fy,
                                name: self.generate_river_name(i),
                                feature_type: "river".to_string(),
                            });
                            placed_labels.push((fx, fy));
                            river_labels_added += 1;
                            break;
                        }
                    }
                }
            }
        }

        labels
    }

    fn find_regions(
        &self,
        terrain: &[Vec<TerrainPoint>],
        predicate: fn(&Biome) -> bool,
    ) -> Vec<Vec<(usize, usize)>> {
        let mut regions = Vec::new();
        let mut visited = vec![vec![false; terrain[0].len()]; terrain.len()];

        for y in 0..terrain.len() {
            for x in 0..terrain[0].len() {
                if !visited[y][x] && predicate(&terrain[y][x].biome) {
                    let mut region = Vec::new();
                    let mut stack = vec![(x, y)];

                    while let Some((cx, cy)) = stack.pop() {
                        if visited[cy][cx] {
                            continue;
                        }

                        visited[cy][cx] = true;
                        region.push((cx, cy));

                        for dy in -1i32..=1 {
                            for dx in -1i32..=1 {
                                if dx == 0 && dy == 0 {
                                    continue;
                                }

                                let nx = cx as i32 + dx;
                                let ny = cy as i32 + dy;

                                if nx >= 0
                                    && nx < terrain[0].len() as i32
                                    && ny >= 0
                                    && ny < terrain.len() as i32
                                {
                                    let nx = nx as usize;
                                    let ny = ny as usize;

                                    if !visited[ny][nx] && predicate(&terrain[ny][nx].biome) {
                                        stack.push((nx, ny));
                                    }
                                }
                            }
                        }
                    }

                    if region.len() > 10 {
                        regions.push(region);
                    }
                }
            }
        }

        regions
    }

    /// The most interior point of a region (pole of inaccessibility): a
    /// multi-source BFS from the region boundary inward, returning the tile
    /// with the greatest distance from any edge. This keeps ocean labels in
    /// open water and mountain labels on the range's core. Tiles outside the
    /// map count as boundary, so labels also stay away from map edges.
    fn region_center(&self, region: &[(usize, usize)]) -> (usize, usize) {
        use std::collections::{HashMap, VecDeque};

        let in_region: std::collections::HashSet<(usize, usize)> =
            region.iter().copied().collect();
        let neighbors = |x: usize, y: usize| {
            [(0i32, -1i32), (-1, 0), (1, 0), (0, 1)]
                .into_iter()
                .map(move |(dx, dy)| (x as i32 + dx, y as i32 + dy))
        };

        // Seed the BFS with all boundary tiles (those with a 4-neighbor
        // outside the region or outside the map)
        let mut dist: HashMap<(usize, usize), u32> = HashMap::new();
        let mut queue = VecDeque::new();
        for &(x, y) in region {
            let on_boundary = neighbors(x, y).any(|(nx, ny)| {
                nx < 0 || ny < 0 || !in_region.contains(&(nx as usize, ny as usize))
            });
            if on_boundary {
                dist.insert((x, y), 0);
                queue.push_back((x, y));
            }
        }

        let mut best = region[0];
        let mut best_dist = 0;
        while let Some((x, y)) = queue.pop_front() {
            let d = dist[&(x, y)];
            if d > best_dist {
                best_dist = d;
                best = (x, y);
            }
            for (nx, ny) in neighbors(x, y) {
                if nx < 0 || ny < 0 {
                    continue;
                }
                let p = (nx as usize, ny as usize);
                if in_region.contains(&p) && !dist.contains_key(&p) {
                    dist.insert(p, d + 1);
                    queue.push_back(p);
                }
            }
        }

        best
    }
}

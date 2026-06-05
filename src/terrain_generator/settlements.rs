use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

use rand::Rng;

use super::biome::Biome;
use super::types::{Bridge, City, Road, TerrainPoint};
use super::TerrainGenerator;

/// Node in the pathfinding priority queue. Ordered by `f` (estimated total
/// cost) so the BinaryHeap acts as a min-heap.
#[derive(Copy, Clone, Eq, PartialEq)]
struct PathState {
    f: usize,
    g: usize,
    position: (usize, usize),
}

impl Ord for PathState {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f.cmp(&self.f)
    }
}

impl PartialOrd for PathState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Walk the came_from chain back from `end` and return the path in
/// start-to-end order.
fn reconstruct_path(
    came_from: &HashMap<(usize, usize), (usize, usize)>,
    end: (usize, usize),
) -> Vec<(usize, usize)> {
    let mut path = vec![end];
    let mut current = end;
    while let Some(&prev) = came_from.get(&current) {
        path.push(prev);
        current = prev;
    }
    path.reverse();
    path
}

impl TerrainGenerator {
    pub(super) fn generate_cities(&mut self, terrain: &[Vec<TerrainPoint>]) -> Vec<City> {
        let mut cities = Vec::new();

        // Handle 0% case - no cities at all
        if self.settings.city_density < 0.01 {
            return cities;
        }

        // First, find all valid land tiles for city placement. Cities can sit
        // on any stable land biome, including the coast (coastal cities are
        // common) - the biome match itself guarantees we're not in water.
        let mut valid_positions = Vec::new();
        for y in 2..terrain.len() - 2 {
            for x in 2..terrain[0].len() - 2 {
                if matches!(
                    terrain[y][x].biome,
                    Biome::Plains | Biome::Hills | Biome::Forest | Biome::Desert | Biome::Beach
                ) {
                    valid_positions.push((x, y));
                }
            }
        }

        if valid_positions.is_empty() {
            return cities;
        }

        // Scale city counts based on settings and available land
        let land_factor = valid_positions.len() as f32 / (terrain.len() * terrain[0].len()) as f32;

        // Major cities: 0-10 based on density and available land
        let num_major_cities = if self.settings.city_density < 0.1 {
            0
        } else {
            let base = ((self.settings.city_density - 0.1) * 10.0 * land_factor * 2.0) as usize;
            base.min(10)
        };

        // Medium cities: 0-25 based on density
        let num_medium_cities = if self.settings.city_density < 0.05 {
            0
        } else {
            let base = ((self.settings.city_density - 0.05) * 25.0 * land_factor * 2.0) as usize;
            base.min(25)
        };

        // Small towns: 0-70 based on density
        let num_towns = {
            let base = (self.settings.city_density * 70.0 * land_factor * 2.0) as usize;
            base.min(70)
        };

        let mut placed_positions = Vec::new();

        // Generate city populations following Zipf's law for major cities
        let base_population = 500000;
        let mut populations: Vec<u32> = Vec::new();

        // Major cities
        for i in 1..=num_major_cities {
            populations.push((base_population as f64 / i as f64) as u32);
        }
        // Medium cities
        for _ in 0..num_medium_cities {
            populations.push(self.rng.gen_range(50000..150000));
        }
        // Small towns
        for _ in 0..num_towns {
            populations.push(self.rng.gen_range(5000..30000));
        }

        // Place cities on suitable terrain with better spacing
        for (idx, pop) in populations.iter().enumerate() {
            let mut attempts = 0;
            let is_major = idx < num_major_cities;
            let is_medium = idx < (num_major_cities + num_medium_cities);

            while attempts < 150 && !valid_positions.is_empty() {
                // Pick from valid land positions
                let pos_idx = self.rng.gen_range(0..valid_positions.len());
                let (x, y) = valid_positions[pos_idx];

                let point = &terrain[y][x];

                // Cities prefer certain terrain types
                let suitable = match point.biome {
                    Biome::Plains => true,
                    Biome::Beach => is_major || self.rng.gen_bool(0.7), // Major cities like coasts
                    Biome::Hills => self.rng.gen_bool(0.5),
                    Biome::Forest => self.rng.gen_bool(0.2),
                    _ => false,
                };

                if !suitable {
                    attempts += 1;
                    continue;
                }

                // Much larger minimum distances for better distribution
                let mut min_dist = if is_major {
                    100.0 // Major cities need LOTS of space
                } else if is_medium {
                    60.0 // Medium cities need good spacing
                } else {
                    40.0 // Towns should also be well-spaced
                };

                // Check for grid alignment and minimum distances
                let mut too_close = false;
                let mut grid_aligned = false;

                for (i, &(cx, cy)) in placed_positions.iter().enumerate() {
                    let dx = x as f64 - cx as f64;
                    let dy = y as f64 - cy as f64;
                    let dist = (dx * dx + dy * dy).sqrt();

                    // Prevent cities from lining up on same latitude/longitude
                    if (dx.abs() < 3.0 || dy.abs() < 3.0) && dist < 40.0 {
                        grid_aligned = true; // Too aligned with existing city
                        break;
                    }

                    // Special case: allow 1-2 towns near major cities (suburbs)
                    if !is_major && !is_medium && i < num_major_cities {
                        // Towns can be closer to major cities (suburbs)
                        if dist < 6.0 {
                            too_close = true; // But not too close
                            break;
                        } else if dist < 12.0 && self.rng.gen_bool(0.3) {
                            // 30% chance to allow suburb placement
                            min_dist = 8.0;
                        }
                    }

                    if dist < min_dist {
                        too_close = true;
                        break;
                    }
                }

                // Add some offset to prevent grid patterns
                if grid_aligned && attempts < 100 {
                    attempts += 1;
                    continue;
                }

                if !too_close {
                    cities.push(City {
                        x,
                        y,
                        name: self.generate_city_name(cities.len()),
                        population: *pop,
                    });
                    placed_positions.push((x, y));
                    break;
                }
                attempts += 1;
            }
        }

        cities
    }

    /// Collect bridges where a road path crosses a river, appending them to
    /// the map-wide bridge list and returning the ones for this road.
    fn detect_bridges(
        &mut self,
        path: &[(usize, usize)],
        river_points: &HashSet<(usize, usize)>,
        terrain: &[Vec<TerrainPoint>],
        all_bridges: &mut Vec<Bridge>,
    ) -> Vec<Bridge> {
        let mut bridges = Vec::new();
        for &(x, y) in path {
            if river_points.contains(&(x, y)) || terrain[y][x].biome == Biome::River {
                let bridge = Bridge {
                    x,
                    y,
                    name: self.generate_bridge_name(all_bridges.len()),
                };
                bridges.push(bridge.clone());
                all_bridges.push(bridge);
            }
        }
        bridges
    }

    pub(super) fn generate_roads(
        &mut self,
        terrain: &[Vec<TerrainPoint>],
        cities: &[City],
        rivers: &[Vec<(usize, usize)>],
    ) -> (Vec<Road>, Vec<Bridge>) {
        let mut roads = Vec::new();
        let mut all_bridges = Vec::new();

        if cities.is_empty() {
            return (roads, all_bridges);
        }

        // Create a set of river points for quick lookup
        let mut river_points = HashSet::new();
        for river in rivers {
            for &point in river {
                river_points.insert(point);
            }
        }

        // Track which cities are connected and existing road points for reuse
        let mut connected_cities = vec![false; cities.len()];
        let mut road_network: HashMap<(usize, usize), Vec<usize>> = HashMap::new();

        // Step 1: Create a minimum spanning tree for major cities to avoid parallel roads
        let major_count = cities.len().min(8);
        let mut mst_edges = Vec::new();

        if major_count > 1 {
            // Calculate all distances between major cities
            let mut edges = Vec::new();
            for i in 0..major_count {
                for j in i + 1..major_count {
                    let dx = cities[i].x as f64 - cities[j].x as f64;
                    let dy = cities[i].y as f64 - cities[j].y as f64;
                    let dist = (dx * dx + dy * dy).sqrt();
                    edges.push((dist, i, j));
                }
            }
            edges.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

            // Build MST using Kruskal's algorithm
            let mut union_find = (0..major_count).collect::<Vec<_>>();
            let find = |uf: &mut Vec<usize>, mut x: usize| -> usize {
                while uf[x] != x {
                    x = uf[x];
                }
                x
            };

            for (dist, i, j) in edges {
                if dist > 80.0 {
                    break; // Don't connect very distant cities
                }

                let root_i = find(&mut union_find, i);
                let root_j = find(&mut union_find, j);

                if root_i != root_j {
                    union_find[root_i] = root_j;
                    mst_edges.push((i, j));
                }
            }
        }

        // Step 2: Build main highways along MST edges
        for (i, j) in mst_edges {
            let path = self.find_path(terrain, cities[i].x, cities[i].y, cities[j].x, cities[j].y);
            if !path.is_empty() {
                connected_cities[i] = true;
                connected_cities[j] = true;

                // Store road segments for potential reuse
                for &point in &path {
                    road_network.entry(point).or_default().push(roads.len());
                }

                let bridges = self.detect_bridges(&path, &river_points, terrain, &mut all_bridges);

                roads.push(Road {
                    path,
                    name: format!("{} Highway", self.generate_road_name(roads.len())),
                    road_type: "highway".to_string(),
                    bridges,
                });
            }
        }

        // Step 3: Connect remaining cities, trying to create Y-junctions by connecting to existing roads
        for i in 0..cities.len() {
            if !connected_cities[i] {
                // Try to find the nearest point on an existing road
                let mut best_connection = None;
                let mut min_cost = f64::MAX;

                // First check if we can connect to an existing road network.
                // HashMap iteration order is random per process, so break
                // distance ties by coordinate to keep generation
                // deterministic for a given seed.
                for &(rx, ry) in road_network.keys() {
                    let dx = cities[i].x as f64 - rx as f64;
                    let dy = cities[i].y as f64 - ry as f64;
                    let dist = (dx * dx + dy * dy).sqrt();

                    // Only consider reasonably close road points
                    if dist < 30.0
                        && (dist < min_cost
                            || (dist == min_cost
                                && best_connection
                                    .map_or(true, |(bx, by, _)| (rx, ry) < (bx, by))))
                    {
                        min_cost = dist;
                        best_connection = Some((rx, ry, true)); // true = connect to road
                    }
                }

                // If no good road connection, find nearest connected city
                if best_connection.is_none() {
                    for j in 0..cities.len() {
                        if i != j && connected_cities[j] {
                            let dx = cities[i].x as f64 - cities[j].x as f64;
                            let dy = cities[i].y as f64 - cities[j].y as f64;
                            let dist = (dx * dx + dy * dy).sqrt();

                            if dist < min_cost {
                                min_cost = dist;
                                best_connection = Some((cities[j].x, cities[j].y, false)); // false = connect to city
                            }
                        }
                    }
                }

                // If still no connection, connect to nearest city regardless
                if best_connection.is_none() {
                    let mut nearest = 0;
                    let mut min_dist = f64::MAX;
                    for j in 0..cities.len() {
                        if i != j {
                            let dx = cities[i].x as f64 - cities[j].x as f64;
                            let dy = cities[i].y as f64 - cities[j].y as f64;
                            let dist = (dx * dx + dy * dy).sqrt();
                            if dist < min_dist {
                                min_dist = dist;
                                nearest = j;
                            }
                        }
                    }
                    best_connection = Some((cities[nearest].x, cities[nearest].y, false));
                }

                if let Some((target_x, target_y, is_road_junction)) = best_connection {
                    let path = self.find_path(terrain, cities[i].x, cities[i].y, target_x, target_y);
                    if !path.is_empty() {
                        connected_cities[i] = true;

                        // Store new road segments
                        for &point in &path {
                            road_network.entry(point).or_default().push(roads.len());
                        }

                        let bridges =
                            self.detect_bridges(&path, &river_points, terrain, &mut all_bridges);

                        let road_type = if cities[i].population > 100000 {
                            "road"
                        } else {
                            "trail"
                        };

                        let road_name = if is_road_junction {
                            format!("{} Branch", self.generate_road_name(roads.len()))
                        } else {
                            format!(
                                "{} {}",
                                self.generate_road_name(roads.len()),
                                if road_type == "trail" { "Trail" } else { "Road" }
                            )
                        };

                        roads.push(Road {
                            path,
                            name: road_name,
                            road_type: road_type.to_string(),
                            bridges,
                        });
                    }
                }
            }
        }

        // Add some partial roads from cities that just go into the wilderness
        for i in 0..cities.len() {
            if self.rng.gen_bool(0.3) {
                // 30% chance for each city to have an extra road
                // Pick a random direction and distance
                let angle = self.rng.gen_range(0.0..std::f64::consts::TAU);
                let distance = self.rng.gen_range(15.0..30.0);

                let target_x = (cities[i].x as f64 + angle.cos() * distance) as usize;
                let target_y = (cities[i].y as f64 + angle.sin() * distance) as usize;

                if target_x < terrain[0].len() && target_y < terrain.len() {
                    // Generate a partial path that might not reach the target
                    let path =
                        self.find_partial_path(terrain, cities[i].x, cities[i].y, target_x, target_y);
                    if path.len() > 5 {
                        // Only add if it's a meaningful path
                        let bridges =
                            self.detect_bridges(&path, &river_points, terrain, &mut all_bridges);

                        roads.push(Road {
                            path,
                            name: format!("Old {} Trail", self.generate_road_name(roads.len())),
                            road_type: "trail".to_string(),
                            bridges,
                        });
                    }
                }
            }
        }

        (roads, all_bridges)
    }

    /// A* pathfinding that avoids water bodies but can cross rivers.
    fn find_path(
        &mut self,
        terrain: &[Vec<TerrainPoint>],
        x1: usize,
        y1: usize,
        x2: usize,
        y2: usize,
    ) -> Vec<(usize, usize)> {
        let mut g_score: HashMap<(usize, usize), usize> = HashMap::new();
        let mut heap = BinaryHeap::new();
        let mut came_from: HashMap<(usize, usize), (usize, usize)> = HashMap::new();

        g_score.insert((x1, y1), 0);
        heap.push(PathState {
            f: 0,
            g: 0,
            position: (x1, y1),
        });

        while let Some(PathState { g, position, .. }) = heap.pop() {
            let (x, y) = position;

            if position == (x2, y2) {
                // Smooth the path to make it more natural
                let path = reconstruct_path(&came_from, (x2, y2));
                return self.smooth_path(path, terrain);
            }

            if g > *g_score.get(&position).unwrap_or(&usize::MAX) {
                continue;
            }

            // Check all 8 neighbors
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }

                    let nx = (x as i32 + dx) as usize;
                    let ny = (y as i32 + dy) as usize;

                    if nx >= terrain[0].len() || ny >= terrain.len() {
                        continue;
                    }

                    let next_terrain = &terrain[ny][nx];

                    // Cannot cross oceans or lakes
                    if matches!(
                        next_terrain.biome,
                        Biome::Ocean | Biome::DeepOcean | Biome::Lake | Biome::Shore
                    ) {
                        continue;
                    }

                    // Calculate cost - consider elevation changes and terrain type
                    let is_diagonal = dx.abs() + dy.abs() == 2;
                    let mut move_cost = if is_diagonal { 14 } else { 10 };

                    // Add cost for elevation changes (roads prefer flat terrain)
                    let current_elevation = terrain[y][x].elevation;
                    let next_elevation = next_terrain.elevation;
                    let elevation_change = (next_elevation - current_elevation).abs();

                    // Heavy penalty for elevation changes
                    move_cost += (elevation_change * 100.0) as usize;

                    // Additional terrain-based costs
                    match next_terrain.biome {
                        Biome::River => move_cost *= 5, // Rivers are expensive to cross (bridges needed)
                        Biome::Mountains => move_cost *= 8, // Mountains are very hard to cross
                        Biome::SnowPeaks => move_cost *= 10, // Snow peaks are nearly impassable
                        Biome::Hills => move_cost *= 2, // Hills are moderately difficult
                        Biome::Swamp => move_cost *= 3, // Swamps are difficult
                        Biome::Forest => move_cost = (move_cost as f32 * 1.5) as usize, // Forests slow travel
                        _ => {}
                    }

                    // Add MORE random variation to prevent unnaturally straight lines
                    move_cost += self.rng.gen_range(5..35);

                    // Shape penalties: discourage right-angle turns and long
                    // straight runs in ANY direction so roads curve gently
                    if let Some(&prev_pos) = came_from.get(&position) {
                        let prev_dx = x as i32 - prev_pos.0 as i32;
                        let prev_dy = y as i32 - prev_pos.1 as i32;

                        // Detect right angles (90-degree turns)
                        let is_right_angle = !is_diagonal
                            && (
                                // From diagonal to straight
                                ((prev_dx != 0 && prev_dy != 0) && (dx == 0 || dy == 0)) ||
                                // From horizontal to vertical
                                (prev_dx != 0 && prev_dy == 0 && dx == 0 && dy != 0) ||
                                // From vertical to horizontal
                                (prev_dx == 0 && prev_dy != 0 && dx != 0 && dy == 0)
                            );

                        if is_right_angle {
                            // PROHIBITIVE penalty for creating right angles
                            move_cost += 1000;
                        } else {
                            // Count consecutive moves in the exact same
                            // direction (diagonals included - otherwise roads
                            // become long 45-degree lines)
                            let mut straight_count = 0;
                            let mut check_pos = position;
                            while let Some(&prev) = came_from.get(&check_pos) {
                                let check_dx = check_pos.0 as i32 - prev.0 as i32;
                                let check_dy = check_pos.1 as i32 - prev.1 as i32;
                                if check_dx == dx && check_dy == dy {
                                    straight_count += 1;
                                    check_pos = prev;
                                } else {
                                    break;
                                }
                            }

                            // Quadratic penalty for straight lines
                            if straight_count > 1 {
                                move_cost += straight_count * straight_count * 30;
                            }

                            // Base penalty for horizontal/vertical movement
                            if dx == 0 || dy == 0 {
                                move_cost += 100 + self.rng.gen_range(25..50);

                                // Extra penalty if continuing horizontal/vertical
                                if (dx == 0 && prev_dx == 0) || (dy == 0 && prev_dy == 0) {
                                    move_cost += 150;
                                }
                            } else if prev_dx != 0 && prev_dy != 0 {
                                // Gentle diagonal-to-diagonal transition bonus
                                let angle_change = (dx - prev_dx).abs() + (dy - prev_dy).abs();
                                if angle_change <= 1 {
                                    move_cost = (move_cost as f32 * 0.85) as usize;
                                }
                            }
                        }
                    } else if dx == 0 || dy == 0 {
                        // First move being horizontal/vertical gets a penalty
                        move_cost += 100;
                    }

                    // Mild preference for diagonal movement
                    if is_diagonal {
                        move_cost = (move_cost as f32 * 0.8) as usize;
                    }

                    // Prefer following contours (moving along similar elevation)
                    if elevation_change < 0.05 {
                        move_cost = (move_cost as f32 * 0.85) as usize;
                    }

                    let next_g = g + move_cost;
                    if next_g < *g_score.get(&(nx, ny)).unwrap_or(&usize::MAX) {
                        g_score.insert((nx, ny), next_g);
                        came_from.insert((nx, ny), position);

                        // Euclidean-distance heuristic, scaled below the base
                        // move cost so it stays admissible. It is added to the
                        // heap priority only - never to the stored g score.
                        let dx_goal = nx as f32 - x2 as f32;
                        let dy_goal = ny as f32 - y2 as f32;
                        let h = ((dx_goal * dx_goal + dy_goal * dy_goal).sqrt() * 3.0) as usize;
                        heap.push(PathState {
                            f: next_g + h,
                            g: next_g,
                            position: (nx, ny),
                        });
                    }
                }
            }
        }

        Vec::new() // No path found
    }

    /// Dijkstra variant that stops early once it has wandered far enough or
    /// hits difficult terrain - used for dead-end wilderness trails.
    fn find_partial_path(
        &mut self,
        terrain: &[Vec<TerrainPoint>],
        x1: usize,
        y1: usize,
        x2: usize,
        y2: usize,
    ) -> Vec<(usize, usize)> {
        let mut g_score: HashMap<(usize, usize), usize> = HashMap::new();
        let mut heap = BinaryHeap::new();
        let mut came_from: HashMap<(usize, usize), (usize, usize)> = HashMap::new();
        let max_steps = 50; // Limit path length
        let mut steps = 0;

        g_score.insert((x1, y1), 0);
        heap.push(PathState {
            f: 0,
            g: 0,
            position: (x1, y1),
        });

        while let Some(PathState { g, position, .. }) = heap.pop() {
            let (x, y) = position;
            steps += 1;

            // Stop if we've gone far enough or reached difficult terrain
            if steps > max_steps
                || matches!(
                    terrain[y][x].biome,
                    Biome::Mountains | Biome::SnowPeaks | Biome::Ocean | Biome::DeepOcean
                )
                || position == (x2, y2)
            {
                let path = reconstruct_path(&came_from, position);
                return self.smooth_path(path, terrain);
            }

            if g > *g_score.get(&position).unwrap_or(&usize::MAX) {
                continue;
            }

            // Check neighbors
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }

                    let nx = (x as i32 + dx) as usize;
                    let ny = (y as i32 + dy) as usize;

                    if nx >= terrain[0].len() || ny >= terrain.len() {
                        continue;
                    }

                    let next_terrain = &terrain[ny][nx];

                    // Cannot cross water
                    if matches!(
                        next_terrain.biome,
                        Biome::Ocean | Biome::DeepOcean | Biome::Lake | Biome::Shore
                    ) {
                        continue;
                    }

                    // Calculate cost
                    let mut move_cost = if dx.abs() + dy.abs() == 2 { 14 } else { 10 };
                    let elevation_change = (next_terrain.elevation - terrain[y][x].elevation).abs();
                    move_cost += (elevation_change * 50.0) as usize;

                    let next_g = g + move_cost;
                    if next_g < *g_score.get(&(nx, ny)).unwrap_or(&usize::MAX) {
                        g_score.insert((nx, ny), next_g);
                        came_from.insert((nx, ny), position);
                        heap.push(PathState {
                            f: next_g,
                            g: next_g,
                            position: (nx, ny),
                        });
                    }
                }
            }
        }

        Vec::new()
    }

    fn smooth_path(
        &mut self,
        path: Vec<(usize, usize)>,
        terrain: &[Vec<TerrainPoint>],
    ) -> Vec<(usize, usize)> {
        if path.len() < 3 {
            return path;
        }

        // First pass: Smooth out sharp corners with larger radius curves
        let mut splined = Vec::new();
        splined.push(path[0]);

        for i in 1..path.len() - 1 {
            let prev = path[i - 1];
            let curr = path[i];
            let next = path[i + 1];

            // Calculate vectors
            let v1x = curr.0 as f32 - prev.0 as f32;
            let v1y = curr.1 as f32 - prev.1 as f32;
            let v2x = next.0 as f32 - curr.0 as f32;
            let v2y = next.1 as f32 - curr.1 as f32;

            // Calculate the dot product to detect angle changes
            let dot_product = v1x * v2x + v1y * v2y;
            let v1_len = (v1x * v1x + v1y * v1y).sqrt();
            let v2_len = (v2x * v2x + v2y * v2y).sqrt();

            // Normalize and calculate angle
            let cos_angle = if v1_len > 0.0 && v2_len > 0.0 {
                dot_product / (v1_len * v2_len)
            } else {
                1.0
            };

            // Detect right angles and sharp turns
            let is_right_angle = cos_angle.abs() < 0.1; // ~90 degrees
            let is_sharp_turn = cos_angle < 0.5; // More than 60 degrees

            // Always smooth any significant direction change
            if is_right_angle || is_sharp_turn || dot_product <= 0.5 {
                // Calculate curve radius based on the angle
                let curve_radius = v1_len.min(v2_len) * 0.4; // Use 40% of the shorter segment

                // Create control points for a smooth curve
                let t1 = curve_radius / v1_len;
                let t2 = curve_radius / v2_len;

                let p1x = curr.0 as f32 - v1x * t1;
                let p1y = curr.1 as f32 - v1y * t1;
                let p2x = curr.0 as f32 + v2x * t2;
                let p2y = curr.1 as f32 + v2y * t2;

                // Generate curve points using cubic Bezier
                for j in 0..=8 {
                    let t = j as f32 / 8.0;
                    let t2_pow = t * t;
                    let t3 = t2_pow * t;
                    let mt = 1.0 - t;
                    let mt2 = mt * mt;
                    let mt3 = mt2 * mt;

                    // Cubic Bezier formula (both inner control points at the corner)
                    let px = mt3 * p1x
                        + 3.0 * mt2 * t * curr.0 as f32
                        + 3.0 * mt * t2_pow * curr.0 as f32
                        + t3 * p2x;
                    let py = mt3 * p1y
                        + 3.0 * mt2 * t * curr.1 as f32
                        + 3.0 * mt * t2_pow * curr.1 as f32
                        + t3 * p2y;

                    let curved_x = px.round().max(0.0) as usize;
                    let curved_y = py.round().max(0.0) as usize;

                    if curved_x < terrain[0].len() && curved_y < terrain.len() {
                        // Check it's not water
                        if !matches!(
                            terrain[curved_y][curved_x].biome,
                            Biome::Ocean | Biome::DeepOcean | Biome::Lake
                        ) {
                            splined.push((curved_x, curved_y));
                        }
                    }
                }
            } else {
                // Keep the original point for straight segments
                splined.push(curr);
            }
        }

        splined.push(path[path.len() - 1]);

        // Second pass: Add natural wiggles to straight segments
        let mut smoothed = Vec::new();
        smoothed.push(splined[0]);

        for i in 0..splined.len() - 1 {
            let current = splined[i];
            let next = splined[i + 1];

            let dx = next.0 as f32 - current.0 as f32;
            let dy = next.1 as f32 - current.1 as f32;
            let distance = (dx * dx + dy * dy).sqrt();

            // For segments longer than 1 unit, add subtle wiggles
            if distance > 1.5 {
                let num_points = (distance * 0.8) as usize;

                // Generate a smooth noise curve for this segment
                let phase = self.rng.gen_range(0.0..std::f32::consts::TAU);
                let frequency = self.rng.gen_range(0.3..0.7);
                let amplitude = self.rng.gen_range(0.5..1.2);

                for j in 1..=num_points {
                    let t = j as f32 / (num_points + 1) as f32;

                    // Base position along the line
                    let base_x = current.0 as f32 + dx * t;
                    let base_y = current.1 as f32 + dy * t;

                    // Calculate perpendicular direction
                    let perp_x = -dy / distance;
                    let perp_y = dx / distance;

                    // Add smooth wiggle using multiple sine waves for natural look
                    let wiggle1 = (t * std::f32::consts::PI * frequency + phase).sin() * amplitude;
                    let wiggle2 = (t * std::f32::consts::PI * frequency * 2.3 + phase * 0.7).sin()
                        * amplitude
                        * 0.3;
                    let total_wiggle = wiggle1 + wiggle2;

                    // Apply wiggle perpendicular to the road direction
                    let wiggle_x = base_x + perp_x * total_wiggle;
                    let wiggle_y = base_y + perp_y * total_wiggle;

                    // Add very small random variation for natural imperfection
                    let final_x = (wiggle_x + self.rng.gen_range(-0.1..0.1)).round() as usize;
                    let final_y = (wiggle_y + self.rng.gen_range(-0.1..0.1)).round() as usize;

                    // Ensure the point is valid and preferably not in water
                    if final_x < terrain[0].len() && final_y < terrain.len() {
                        if !matches!(
                            terrain[final_y][final_x].biome,
                            Biome::Ocean | Biome::DeepOcean | Biome::Lake
                        ) {
                            smoothed.push((final_x, final_y));
                        } else {
                            // Fall back to straight line if curve goes into water
                            smoothed.push((base_x.round() as usize, base_y.round() as usize));
                        }
                    }
                }
            }

            smoothed.push(next);
        }

        // Remove any duplicate consecutive points
        smoothed.dedup();
        smoothed
    }
}

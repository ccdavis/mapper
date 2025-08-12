    fn generate_elevation(&self, x: usize, y: usize, width: usize, height: usize) -> f64 {
        // Normalize coordinates to [0, 1]
        let nx = x as f64 / width as f64;
        let ny = y as f64 / height as f64;
        
        let land_target = self.settings.land_percentage as f64;
        
        // Decide if this map should have continents reaching edges (25% chance)
        // Use a deterministic decision based on seed
        let seed_hash = (self.continent_noise.get([0.123, 0.456]) * 1000.0) as i32;
        let has_edge_continent = (seed_hash.abs() % 100) < 25;
        
        // CONTINENT GENERATION - Create 1-3 major landmasses
        let num_continents = if land_target < 0.3 { 1 } 
                           else if land_target < 0.6 { 2 } 
                           else { 3 };
        
        let mut continent_value = -1.0; // Start with ocean
        
        for i in 0..num_continents {
            // Each continent has a center point
            let offset = i as f64 * 100.0;
            
            // Continent centers - use noise to place them
            let cx = if has_edge_continent && i == 0 {
                // First continent can be at edge
                if self.continent_noise.get([offset, offset]) > 0.0 { 
                    self.rng.gen_range(-0.2..0.2) // Left edge
                } else { 
                    0.8 + self.rng.gen_range(0.0..0.2) // Right edge
                }
            } else {
                // Place continent center away from edges
                0.25 + (self.continent_noise.get([offset * 0.1, offset * 0.1]) + 1.0) * 0.25
            };
            
            let cy = if has_edge_continent && i == 0 {
                0.5 + self.continent_noise.get([offset * 0.2, offset * 0.2]) * 0.3
            } else {
                0.25 + (self.continent_noise.get([offset * 0.1 + 10.0, offset * 0.1 + 10.0]) + 1.0) * 0.25
            };
            
            // Distance from this continent's center
            let dx = nx - cx;
            let dy = ny - cy;
            let dist_from_center = (dx * dx + dy * dy).sqrt();
            
            // Continent size based on land percentage
            let base_size = 0.25 + land_target * 0.35; // Radius from 0.25 to 0.6
            let size_variation = self.continent_noise.get([offset * 0.05, offset * 0.05]) * 0.15;
            let continent_radius = (base_size + size_variation).max(0.2).min(0.8);
            
            // Create irregular continent shape
            let angle = dy.atan2(dx);
            
            // Use multiple noise layers for realistic continent shape
            let shape1 = self.continent_noise.get([angle * 2.0 + offset, offset]) * 0.3;
            let shape2 = self.elevation_noise.get([angle * 4.0 + offset, offset * 0.5]) * 0.15;
            let shape3 = self.detail_noise.get([nx * 5.0 + offset, ny * 5.0 + offset]) * 0.1;
            
            let shape_modifier = shape1 + shape2 + shape3;
            let modified_radius = continent_radius * (1.0 + shape_modifier);
            
            // Calculate elevation based on distance from center
            let continent_elevation = if dist_from_center < modified_radius * 0.3 {
                // Inner continent - high elevation
                0.8 + self.elevation_noise.get([nx * 10.0, ny * 10.0]) * 0.2
            } else if dist_from_center < modified_radius * 0.6 {
                // Mid continent
                0.4 + self.elevation_noise.get([nx * 10.0, ny * 10.0]) * 0.3
            } else if dist_from_center < modified_radius {
                // Continental shelf - gradual falloff
                let falloff = (modified_radius - dist_from_center) / (modified_radius * 0.4);
                falloff * 0.4 - 0.2
            } else {
                // Deep ocean
                -0.8 - (dist_from_center - modified_radius).min(0.5)
            };
            
            // Take maximum to combine overlapping continents
            continent_value = continent_value.max(continent_elevation);
        }
        
        // Add smaller islands based on land percentage
        if land_target > 0.2 {
            let island_chains = self.elevation_noise.get([nx * 12.0, ny * 12.0]) * 0.8 
                              + self.detail_noise.get([nx * 20.0, ny * 20.0]) * 0.3;
            if island_chains > 0.5 {
                continent_value = continent_value.max(island_chains - 0.4);
            }
        }
        
        // Apply edge falloff only if not an edge continent
        if !has_edge_continent && continent_value > -0.3 {
            let edge_dist_x = (0.5 - (nx - 0.5).abs()) * 2.0;
            let edge_dist_y = (0.5 - (ny - 0.5).abs()) * 2.0;
            let edge_factor = edge_dist_x.min(edge_dist_y);
            
            if edge_factor < 0.15 {
                continent_value = continent_value * (edge_factor / 0.15) - (1.0 - edge_factor / 0.15);
            }
        }
        
        // Add detail for realistic coastlines
        let coastline = self.elevation_noise.get([nx * 30.0, ny * 30.0]) * 0.12
                      + self.detail_noise.get([nx * 60.0, ny * 60.0]) * 0.06;
        
        let base_elevation = continent_value + coastline;
        
        // Determine sea level based on land percentage
        // This is the critical value that determines how much land appears
        let sea_level = -0.2 + (1.0 - land_target) * 0.4;
        
        // Convert to final elevation with clear land/water boundary
        let elevation = if base_elevation > sea_level {
            // Land
            ((base_elevation - sea_level) / (1.0 - sea_level)).min(1.0).max(0.0)
        } else {
            // Water
            ((base_elevation - sea_level) / (1.0 + sea_level)).max(-1.0)
        };
        
        elevation
    }
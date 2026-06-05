use crate::terrain_generator::{Biome, TerrainMap};
use image::{ImageBuffer, Rgb, RgbImage};

pub struct TerrainRenderer;

/// Smooth ocean gradient from abyss (elevation -1) to sea level (0).
/// Used instead of discrete biome colors so the water shows no banding.
fn water_color(elevation: f64) -> [f32; 3] {
    let t = ((elevation + 1.0).clamp(0.0, 1.0)) as f32;
    [
        2.0 + t * 28.0,   // R: 2 -> 30
        18.0 + t * 72.0,  // G: 18 -> 90
        70.0 + t * 110.0, // B: 70 -> 180
    ]
}

impl TerrainRenderer {
    /// Renders a terrain map to RGBA pixel data
    pub fn render_to_pixels(
        map: &TerrainMap,
        width: usize,
        height: usize,
        scale: usize,
    ) -> Vec<u8> {
        let img_width = width * scale;
        let img_height = height * scale;
        let mut pixels = vec![0u8; img_width * img_height * 4];

        if width < 2 || height < 2 {
            return pixels;
        }

        // Bilinear elevation sampling at sub-tile precision (clamped at edges)
        let sample_elevation = |tx: f32, ty: f32| -> f64 {
            let x0 = (tx.max(0.0).floor() as usize).min(width - 2);
            let y0 = (ty.max(0.0).floor() as usize).min(height - 2);
            let fx = ((tx - x0 as f32).clamp(0.0, 1.0)) as f64;
            let fy = ((ty - y0 as f32).clamp(0.0, 1.0)) as f64;

            let e00 = map.terrain[y0][x0].elevation;
            let e10 = map.terrain[y0][x0 + 1].elevation;
            let e01 = map.terrain[y0 + 1][x0].elevation;
            let e11 = map.terrain[y0 + 1][x0 + 1].elevation;

            let e0 = e00 * (1.0 - fx) + e10 * fx;
            let e1 = e01 * (1.0 - fx) + e11 * fx;
            e0 * (1.0 - fy) + e1 * fy
        };

        // Helper function to get terrain color with smooth coastlines
        let get_terrain_color = |x: f32, y: f32| -> [f32; 3] {
            let x0 = (x.max(0.0).floor() as usize).min(width - 1);
            let y0 = (y.max(0.0).floor() as usize).min(height - 1);
            let x1 = (x0 + 1).min(width - 1);
            let y1 = (y0 + 1).min(height - 1);

            let fx = (x - x0 as f32).clamp(0.0, 1.0);
            let fy = (y - y0 as f32).clamp(0.0, 1.0);

            // Get the four corner points
            let get_point_data = |px: usize, py: usize| -> ([f32; 3], bool) {
                let terrain_point = &map.terrain[py][px];
                let is_water = terrain_point.biome.is_water();
                let color = if terrain_point.biome == Biome::Lake {
                    let c = terrain_point.biome.color();
                    [c[0] as f32, c[1] as f32, c[2] as f32]
                } else if is_water {
                    // Smooth gradient for oceans - no biome banding
                    water_color(terrain_point.elevation)
                } else {
                    let base_color = Biome::elevation_color(terrain_point.elevation);
                    let biome_color = terrain_point.biome.color();
                    let blend_factor = 0.7;
                    [
                        base_color[0] as f32 * (1.0 - blend_factor)
                            + biome_color[0] as f32 * blend_factor,
                        base_color[1] as f32 * (1.0 - blend_factor)
                            + biome_color[1] as f32 * blend_factor,
                        base_color[2] as f32 * (1.0 - blend_factor)
                            + biome_color[2] as f32 * blend_factor,
                    ]
                };
                (color, is_water)
            };

            let (c00, water00) = get_point_data(x0, y0);
            let (c10, water10) = get_point_data(x1, y0);
            let (c01, water01) = get_point_data(x0, y1);
            let (c11, water11) = get_point_data(x1, y1);

            // Check if this is a water-land boundary
            let water_count = [water00, water10, water01, water11]
                .iter()
                .filter(|&&w| w)
                .count();

            let bilinear = |c00: [f32; 3], c10: [f32; 3], c01: [f32; 3], c11: [f32; 3]| {
                let mut result = [0.0; 3];
                for i in 0..3 {
                    let c0 = c00[i] * (1.0 - fx) + c10[i] * fx;
                    let c1 = c01[i] * (1.0 - fx) + c11[i] * fx;
                    result[i] = c0 * (1.0 - fy) + c1 * fy;
                }
                result
            };

            // If all same type, use smooth interpolation
            if water_count == 0 || water_count == 4 {
                bilinear(c00, c10, c01, c11)
            } else {
                // Marching-squares-style sharp coastline between the corners
                let v00 = if water00 { 0.0 } else { 1.0 };
                let v10 = if water10 { 0.0 } else { 1.0 };
                let v01 = if water01 { 0.0 } else { 1.0 };
                let v11 = if water11 { 0.0 } else { 1.0 };

                // Bilinear interpolation of the land/water field
                let v0 = v00 * (1.0 - fx) + v10 * fx;
                let v1 = v01 * (1.0 - fx) + v11 * fx;
                let v = v0 * (1.0 - fy) + v1 * fy;

                if v > 0.5 {
                    // Land side - use nearest land color
                    if !water00 {
                        c00
                    } else if !water10 {
                        c10
                    } else if !water01 {
                        c01
                    } else {
                        c11
                    }
                } else {
                    // Water side - use nearest water color
                    if water00 {
                        c00
                    } else if water10 {
                        c10
                    } else if water01 {
                        c01
                    } else {
                        c11
                    }
                }
            }
        };

        // Render each pixel with smooth interpolation
        for py in 0..img_height {
            for px in 0..img_width {
                // Calculate position in terrain space with sub-pixel precision
                let tx = px as f32 / scale as f32;
                let ty = py as f32 / scale as f32;

                let mut color = get_terrain_color(tx, ty);

                let terrain_x = (tx.floor() as usize).min(width - 1);
                let terrain_y = (ty.floor() as usize).min(height - 1);
                let current_terrain = &map.terrain[terrain_y][terrain_x];

                let elev_center = sample_elevation(tx, ty);

                // Hillshade relief on land, from the smoothly interpolated
                // elevation gradient (no screen-space texture patterns)
                if elev_center > 0.0 {
                    let elevation_factor = elev_center.clamp(0.0, 1.0);

                    // Stronger relief at higher elevations, subtle on plains
                    let gradient_scale = if elev_center > 0.82 {
                        25.0 + elevation_factor * 5.0 // Mountains
                    } else if elev_center > 0.6 {
                        15.0 + elevation_factor * 10.0 // Hills
                    } else if elev_center > 0.18 {
                        8.0 + elevation_factor * 7.0 // Uplands
                    } else {
                        3.0 + elevation_factor * 5.0 // Plains
                    };

                    let sample_dist = 0.35;
                    let dx = (sample_elevation(tx + sample_dist, ty)
                        - sample_elevation(tx - sample_dist, ty))
                        * gradient_scale
                        / (sample_dist as f64 * 2.0);
                    let dy = (sample_elevation(tx, ty + sample_dist)
                        - sample_elevation(tx, ty - sample_dist))
                        * gradient_scale
                        / (sample_dist as f64 * 2.0);

                    // Light from the northwest
                    let light = (-0.7071, -0.7071, 0.5);

                    // Surface normal from the gradient
                    let normal_len = (dx * dx + dy * dy + 1.0).sqrt();
                    let lighting = ((-dx) * light.0 + (-dy) * light.1 + light.2).max(0.0)
                        / normal_len;

                    // Moderate contrast: brighter on lit slopes, darker in shade
                    let contrast = 0.3 + elevation_factor as f32 * 0.4;
                    let shade_factor = if lighting > 0.6 {
                        1.0 + (lighting - 0.6) as f32 * contrast
                    } else {
                        0.7 + lighting as f32 * 0.5
                    };

                    color[0] = (color[0] * shade_factor).min(255.0);
                    color[1] = (color[1] * shade_factor).min(255.0);
                    color[2] = (color[2] * shade_factor).min(255.0);

                    // Slight brown tint on steep slopes
                    if dx.abs() > 0.1 || dy.abs() > 0.1 {
                        let slope_intensity = ((dx.abs() + dy.abs()).min(1.0) * 0.1) as f32;
                        color[0] = (color[0] * (1.0 - slope_intensity) + 139.0 * slope_intensity)
                            .min(255.0);
                        color[1] = (color[1] * (1.0 - slope_intensity) + 90.0 * slope_intensity)
                            .min(255.0);
                        color[2] = (color[2] * (1.0 - slope_intensity) + 43.0 * slope_intensity)
                            .min(255.0);
                    }
                }

                // Darken water immediately next to land for a coastline edge
                if current_terrain.biome.is_water() {
                    let mut near_land = false;
                    for dy in -1i32..=1 {
                        for dx in -1i32..=1 {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            let nx = terrain_x as i32 + dx;
                            let ny = terrain_y as i32 + dy;
                            if nx >= 0
                                && ny >= 0
                                && (nx as usize) < width
                                && (ny as usize) < height
                                && !map.terrain[ny as usize][nx as usize].biome.is_water()
                            {
                                near_land = true;
                            }
                        }
                    }
                    if near_land {
                        color[0] *= 0.85;
                        color[1] *= 0.9;
                        color[2] *= 0.95;
                    }
                }

                let pixel_index = (py * img_width + px) * 4;
                pixels[pixel_index] = color[0] as u8;
                pixels[pixel_index + 1] = color[1] as u8;
                pixels[pixel_index + 2] = color[2] as u8;
                pixels[pixel_index + 3] = 255;
            }
        }

        // Draw rivers as tapered lines: narrow at the source, wider at the
        // mouth (rivers are traced source-to-mouth by the generator)
        let river_color = [30.0f32, 100.0, 220.0];
        let scale_f = scale as f32;
        for river in &map.rivers {
            if river.len() < 2 {
                continue;
            }
            for i in 0..river.len() - 1 {
                let t = i as f32 / river.len() as f32;
                let radius = (scale_f * (0.15 + 0.4 * t)).max(0.7);

                let (x0, y0) = river[i];
                let (x1, y1) = river[i + 1];
                let px0 = x0 as f32 * scale_f + scale_f / 2.0;
                let py0 = y0 as f32 * scale_f + scale_f / 2.0;
                let px1 = x1 as f32 * scale_f + scale_f / 2.0;
                let py1 = y1 as f32 * scale_f + scale_f / 2.0;

                let seg_len = ((px1 - px0).powi(2) + (py1 - py0).powi(2)).sqrt();
                let steps = (seg_len.ceil() as usize).max(1);
                for s in 0..=steps {
                    let st = s as f32 / steps as f32;
                    let cx = px0 + (px1 - px0) * st;
                    let cy = py0 + (py1 - py0) * st;

                    let r = radius.ceil() as i32;
                    for dy in -r..=r {
                        for dx in -r..=r {
                            if (dx * dx + dy * dy) as f32 > radius * radius {
                                continue;
                            }
                            let ix = cx as i32 + dx;
                            let iy = cy as i32 + dy;
                            if ix < 0 || iy < 0 || ix >= img_width as i32 || iy >= img_height as i32
                            {
                                continue;
                            }
                            let idx = ((iy as usize) * img_width + ix as usize) * 4;
                            pixels[idx] = river_color[0] as u8;
                            pixels[idx + 1] = river_color[1] as u8;
                            pixels[idx + 2] = river_color[2] as u8;
                        }
                    }
                }
            }
        }

        // Draw roads with better visibility
        for road in &map.roads {
            // Darker, more visible colors
            let (road_color, road_width) = match road.road_type.as_str() {
                "highway" => ([40, 40, 45, 230], 2usize), // Dark gray, 2 pixels wide
                "road" => ([60, 55, 50, 220], 1),         // Dark brown-gray, 1 pixel
                _ => ([80, 70, 60, 200], 1),              // Brown trail, 1 pixel
            };
            let road_blend = road_color[3] as f32 / 255.0;

            let mut draw_road_pixel = |px: usize, py: usize| {
                if px >= img_width || py >= img_height {
                    return;
                }
                let idx = (py * img_width + px) * 4;
                pixels[idx] = (pixels[idx] as f32 * (1.0 - road_blend)
                    + road_color[0] as f32 * road_blend) as u8;
                pixels[idx + 1] = (pixels[idx + 1] as f32 * (1.0 - road_blend)
                    + road_color[1] as f32 * road_blend) as u8;
                pixels[idx + 2] = (pixels[idx + 2] as f32 * (1.0 - road_blend)
                    + road_color[2] as f32 * road_blend) as u8;
            };

            // Draw road path, connecting consecutive points with lines
            for i in 0..road.path.len() {
                let (x, y) = road.path[i];
                if x >= width || y >= height {
                    continue;
                }
                let base_px = x * scale + scale / 2;
                let base_py = y * scale + scale / 2;

                let mut draw_stamp = |px: usize, py: usize| {
                    for offset in 0..road_width {
                        draw_road_pixel(px + offset, py);
                        if road_width == 2 {
                            draw_road_pixel(px + offset, py + 1);
                        }
                    }
                };

                draw_stamp(base_px, base_py);

                // Connect to next point with interpolation for smooth curves
                if i < road.path.len() - 1 {
                    let (next_x, next_y) = road.path[i + 1];
                    if next_x >= width || next_y >= height {
                        continue;
                    }
                    let next_px = next_x * scale + scale / 2;
                    let next_py = next_y * scale + scale / 2;

                    let dx = (next_px as i32 - base_px as i32).abs();
                    let dy = (next_py as i32 - base_py as i32).abs();
                    let steps = dx.max(dy) as usize;

                    for step in 1..steps {
                        let t = step as f32 / steps as f32;
                        let interp_x =
                            (base_px as f32 * (1.0 - t) + next_px as f32 * t) as usize;
                        let interp_y =
                            (base_py as f32 * (1.0 - t) + next_py as f32 * t) as usize;
                        draw_stamp(interp_x, interp_y);
                    }
                }
            }
        }

        // Draw cities as round dots with circles for large cities
        for city in &map.cities {
            let cx = (city.x * scale + scale / 2) as i32;
            let cy = (city.y * scale + scale / 2) as i32;

            // Determine if it's a large city that needs a circle
            let is_large_city = city.population > 100000;

            // City dot sizes - scaled based on tile size for visibility
            let size_factor = (scale as f32 / 10.0).max(0.5); // Scale relative to 10px baseline
            let dot_radius = if city.population > 250000 {
                (12.0 * size_factor) as i32 // Major cities
            } else if city.population > 100000 {
                (9.0 * size_factor) as i32 // Large cities
            } else {
                (6.0 * size_factor) as i32 // Towns
            };

            let mut put_pixel = |px: i32, py: i32, color: [u8; 3]| {
                if px < 0 || py < 0 || px >= img_width as i32 || py >= img_height as i32 {
                    return;
                }
                let idx = ((py as usize) * img_width + px as usize) * 4;
                pixels[idx] = color[0];
                pixels[idx + 1] = color[1];
                pixels[idx + 2] = color[2];
                pixels[idx + 3] = 255;
            };

            // Draw circle around large cities first
            if is_large_city {
                let circle_radius = dot_radius + 3; // Circle 3 pixels larger than dot

                for dy in -(circle_radius + 1)..=(circle_radius + 1) {
                    for dx in -(circle_radius + 1)..=(circle_radius + 1) {
                        let dist_sq = dx * dx + dy * dy;
                        let outer = (circle_radius + 1) * (circle_radius + 1);
                        let inner = (circle_radius - 1) * (circle_radius - 1);

                        // Draw if we're in the circle ring (not inside, not outside)
                        if dist_sq <= outer && dist_sq >= inner {
                            put_pixel(cx + dx, cy + dy, [20, 20, 20]);
                        }
                    }
                }
            }

            // Draw solid round dot for city
            let dot_color = if city.population > 250000 {
                [220, 20, 20] // Major cities - red dot
            } else if city.population > 100000 {
                [180, 40, 40] // Large cities - dark red dot
            } else {
                [20, 20, 20] // Towns - black dot
            };
            for dy in -dot_radius..=dot_radius {
                for dx in -dot_radius..=dot_radius {
                    if dx * dx + dy * dy <= dot_radius * dot_radius {
                        put_pixel(cx + dx, cy + dy, dot_color);
                    }
                }
            }
        }

        pixels
    }

    /// Renders terrain map to an image for PNG export
    pub fn render_to_image(map: &TerrainMap, scale: u32) -> RgbImage {
        let width = map.width as u32 * scale;
        let height = map.height as u32 * scale;
        let mut img = ImageBuffer::new(width, height);

        // Get the pixel data
        let pixels = Self::render_to_pixels(map, map.width, map.height, scale as usize);

        // Convert to RGB image
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                if idx + 2 < pixels.len() {
                    img.put_pixel(x, y, Rgb([pixels[idx], pixels[idx + 1], pixels[idx + 2]]));
                }
            }
        }

        img
    }
}

use noise::NoiseFn;
use rand::Rng;
use rand_chacha::ChaCha8Rng;

use super::types::GenerationSettings;
use super::TerrainGenerator;

/// A soft elliptical bump of elevation. Every continent formation is built
/// from a handful of these; the fractal noise on top supplies all coastline
/// and terrain detail, so the analytic shape never shows through directly.
struct Blob {
    x: f64,      // center, map-normalized [0, 1]
    y: f64,      // center, map-normalized [0, 1]
    radius: f64, // map-normalized radius
    weight: f64, // peak contribution in [0, 1]
    angle: f64,  // orientation of elongation
    aspect: f64, // >1.0 stretches the blob along `angle`
}

impl Blob {
    /// Contribution of this blob at map-normalized (nx, ny): a smooth
    /// Gaussian-like falloff from `weight` at the center to 0 outside.
    fn contribution(&self, nx: f64, ny: f64) -> f64 {
        let dx = nx - self.x;
        let dy = ny - self.y;
        // Rotate into the blob's frame and apply elongation
        let along = (dx * self.angle.cos() + dy * self.angle.sin()) / self.aspect;
        let across = -dx * self.angle.sin() + dy * self.angle.cos();
        let d2 = (along * along + across * across) / (self.radius * self.radius);
        self.weight * (-2.5 * d2).exp()
    }
}

/// The large-scale layout of landmasses for one map, drawn once per
/// generation from the seeded RNG.
pub(super) struct ContinentPlan {
    blobs: Vec<Blob>,
    /// Allow land to touch the map edge (no edge falloff).
    edge_continent: bool,
}

impl ContinentPlan {
    pub(super) fn new(rng: &mut ChaCha8Rng, settings: &GenerationSettings) -> Self {
        let land = settings.land_percentage as f64;
        let mut blobs = Vec::new();

        let formation = rng.gen_range(0..5);
        match formation {
            0 => {
                // Volcanic island chain along a curved line (Hawaii, Aleutians)
                let n = rng.gen_range(5..10);
                let angle = rng.gen_range(0.0..std::f64::consts::TAU);
                let length = rng.gen_range(0.5..0.8);
                let sx = rng.gen_range(0.3..0.7) - angle.cos() * length * 0.5;
                let sy = rng.gen_range(0.3..0.7) - angle.sin() * length * 0.5;
                let bow = rng.gen_range(-0.15..0.15);
                for i in 0..n {
                    let t = i as f64 / (n - 1) as f64;
                    let curve = (t * std::f64::consts::PI).sin() * bow;
                    let mid = 1.0 - (t - 0.5).abs() * 1.2; // bigger islands mid-chain
                    blobs.push(Blob {
                        x: sx + angle.cos() * t * length + angle.sin() * curve
                            + rng.gen_range(-0.03..0.03),
                        y: sy + angle.sin() * t * length - angle.cos() * curve
                            + rng.gen_range(-0.03..0.03),
                        radius: 0.04 + mid.max(0.2) * rng.gen_range(0.02..0.06),
                        weight: rng.gen_range(0.75..1.0),
                        angle: rng.gen_range(0.0..std::f64::consts::TAU),
                        aspect: rng.gen_range(1.0..1.6),
                    });
                }
            }
            1 => {
                // Elongated continent with a mountainous spine
                let angle = rng.gen_range(0.0..std::f64::consts::TAU);
                let cx = rng.gen_range(0.4..0.6);
                let cy = rng.gen_range(0.4..0.6);
                let n = rng.gen_range(4..7);
                for i in 0..n {
                    let t = (i as f64 / (n - 1) as f64) - 0.5;
                    let along = t * rng.gen_range(0.35..0.55);
                    blobs.push(Blob {
                        x: cx + angle.cos() * along + rng.gen_range(-0.04..0.04),
                        y: cy + angle.sin() * along + rng.gen_range(-0.04..0.04),
                        radius: rng.gen_range(0.10..0.18) * (1.0 - t.abs()),
                        weight: rng.gen_range(0.85..1.0),
                        angle,
                        aspect: rng.gen_range(1.8..2.8),
                    });
                }
            }
            2 => {
                // Crescent / island arc (Japan, Indonesia)
                let ccx = rng.gen_range(0.35..0.65);
                let ccy = rng.gen_range(0.35..0.65);
                let arc_r = rng.gen_range(0.22..0.38);
                let start = rng.gen_range(0.0..std::f64::consts::TAU);
                let span = rng.gen_range(1.8..3.5);
                let n = rng.gen_range(6..10);
                for i in 0..n {
                    let theta = start + span * (i as f64 / (n - 1) as f64);
                    blobs.push(Blob {
                        x: ccx + theta.cos() * arc_r + rng.gen_range(-0.02..0.02),
                        y: ccy + theta.sin() * arc_r + rng.gen_range(-0.02..0.02),
                        radius: rng.gen_range(0.06..0.12),
                        weight: rng.gen_range(0.7..1.0),
                        // Elongate along the arc tangent
                        angle: theta + std::f64::consts::FRAC_PI_2,
                        aspect: rng.gen_range(1.4..2.2),
                    });
                }
            }
            3 => {
                // A few large plates forming a complex landmass
                let n = 2 + (land * 2.0) as usize + rng.gen_range(0..2);
                for _ in 0..n {
                    blobs.push(Blob {
                        x: rng.gen_range(0.25..0.75),
                        y: rng.gen_range(0.25..0.75),
                        radius: rng.gen_range(0.16..0.30),
                        weight: rng.gen_range(0.8..1.0),
                        angle: rng.gen_range(0.0..std::f64::consts::TAU),
                        aspect: rng.gen_range(1.0..1.6),
                    });
                }
            }
            _ => {
                // Archipelago: clusters of islands with power-law sizes
                let clusters = rng.gen_range(2..4);
                for _ in 0..clusters {
                    let ccx = rng.gen_range(0.2..0.8);
                    let ccy = rng.gen_range(0.2..0.8);
                    let n = rng.gen_range(4..8);
                    for i in 0..n {
                        let decay = (-(i as f64) / 3.0).exp(); // few large, many small
                        let dist = rng.gen_range(0.02..0.2);
                        let dir = rng.gen_range(0.0..std::f64::consts::TAU);
                        blobs.push(Blob {
                            x: ccx + dir.cos() * dist,
                            y: ccy + dir.sin() * dist,
                            radius: 0.025 + decay * rng.gen_range(0.04..0.09),
                            weight: rng.gen_range(0.6..1.0),
                            angle: rng.gen_range(0.0..std::f64::consts::TAU),
                            aspect: rng.gen_range(1.0..1.8),
                        });
                    }
                }
            }
        }

        // A few outlying islets for every formation type
        for _ in 0..rng.gen_range(2..6) {
            blobs.push(Blob {
                x: rng.gen_range(0.1..0.9),
                y: rng.gen_range(0.1..0.9),
                radius: rng.gen_range(0.015..0.04),
                weight: rng.gen_range(0.4..0.7),
                angle: rng.gen_range(0.0..std::f64::consts::TAU),
                aspect: rng.gen_range(1.0..1.5),
            });
        }

        ContinentPlan {
            blobs,
            edge_continent: rng.gen_bool(0.25),
        }
    }

    /// Land bias in [-0.8, 0.8] at map-normalized (nx, ny): the soft union of
    /// all blob contributions.
    fn bias(&self, nx: f64, ny: f64) -> f64 {
        // Probabilistic union: overlapping blobs merge smoothly instead of
        // stacking, so plate boundaries don't produce walls.
        let mut sea_prob = 1.0;
        for blob in &self.blobs {
            sea_prob *= 1.0 - blob.contribution(nx, ny).clamp(0.0, 1.0);
        }
        let mask = 1.0 - sea_prob; // [0, 1]
        mask * 1.6 - 0.8
    }
}

impl TerrainGenerator {
    /// Generate the full elevation field for the map.
    ///
    /// Elevation is domain-warped fractal noise biased by the continent plan.
    /// The sea level is then chosen as the exact (1 - land_percentage)
    /// quantile of the generated values, so the land/water ratio matches the
    /// settings for every seed and formation type. Returned values are
    /// normalized to [-1, 0) for water and (0, 1] for land.
    pub(super) fn generate_elevation_field(
        &mut self,
        width: usize,
        height: usize,
    ) -> Vec<Vec<f64>> {
        let plan = ContinentPlan::new(&mut self.rng, &self.settings);

        // Isotropic noise coordinates (same frequency on both axes)
        let iso = 1.0 / width.min(height) as f64;

        let mut raw = vec![vec![0.0f64; width]; height];
        for (y, row) in raw.iter_mut().enumerate() {
            for (x, value) in row.iter_mut().enumerate() {
                let nx = x as f64 / width as f64;
                let ny = y as f64 / height as f64;
                let ax = x as f64 * iso;
                let ay = y as f64 * iso;

                // Domain warp: perturb the sample position with low-frequency
                // noise so coastlines and ranges meander instead of following
                // the blob geometry.
                let warp = 0.35;
                let wx = self.detail_noise.get([ax * 2.0 + 31.4, ay * 2.0 + 47.2]);
                let wy = self.detail_noise.get([ax * 2.0 + 73.1, ay * 2.0 + 11.9]);
                let qx = ax + wx * warp;
                let qy = ay + wy * warp;

                // 5-octave fBm for terrain detail
                let mut amp = 1.0;
                let mut freq = 2.0;
                let mut sum = 0.0;
                let mut norm = 0.0;
                for _ in 0..5 {
                    sum += self.elevation_noise.get([qx * freq, qy * freq]) * amp;
                    norm += amp;
                    amp *= 0.5;
                    freq *= 2.0;
                }
                let fbm = sum / norm; // roughly [-1, 1]

                // Ridged noise forms connected mountain ranges instead of
                // isolated round peaks: ridge lines follow the zero-set of a
                // low-frequency noise field.
                let ridge = {
                    let r = 1.0 - self.elevation_noise.get([qx * 3.0 + 113.5, qy * 3.0 + 57.7]).abs();
                    r * r
                };

                // Ridges are weighted by the continent mask so mountain
                // ranges form on continent cores, not in open ocean.
                let bias = plan.bias(nx, ny);
                let mask01 = (bias + 0.8) / 1.6;
                let mut v = bias + fbm * 0.45 + ridge * 0.5 * mask01;

                // Soft edge falloff keeps continents off the map border
                // (75% of maps) so coastlines don't get clipped.
                if !plan.edge_continent {
                    let edge = (nx.min(1.0 - nx)).min(ny.min(1.0 - ny));
                    let f = (edge / 0.08).clamp(0.0, 1.0);
                    let f = f * f * (3.0 - 2.0 * f); // smoothstep
                    v = v * f - (1.0 - f);
                }

                *value = v;
            }
        }

        // Histogram-equalize the field: each tile's elevation becomes its
        // area quantile. Sea level sits at exactly (1 - land_percentage), so
        // the land/water ratio matches the settings for every seed, and the
        // biome thresholds in `determine_biome` directly control what share
        // of the land each biome covers.
        let mut sorted: Vec<f64> = raw.iter().flatten().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let land = self.settings.land_percentage as f64;
        let sea_idx = (((1.0 - land) * (sorted.len() - 1) as f64) as usize).min(sorted.len() - 1);
        let sea_level = sorted[sea_idx];
        let land_count = (sorted.len() - 1 - sea_idx).max(1) as f64;
        let water_count = sea_idx.max(1) as f64;

        for row in raw.iter_mut() {
            for value in row.iter_mut() {
                // Rank of this value in the sorted field (binary search)
                let rank = sorted.partition_point(|v| *v < *value);
                *value = if *value > sea_level {
                    // Land: quantile within land, in (0, 1]
                    (((rank - sea_idx) as f64) / land_count).clamp(0.01, 1.0)
                } else {
                    // Water: quantile within water, in [-1, 0)
                    ((rank as f64 / water_count) - 1.0).min(-0.01)
                };
            }
        }

        raw
    }
}

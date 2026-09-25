use std::collections::{HashMap, HashSet};

use crate::noise;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TileKey {
    pub face: u8,
    pub x: u32,
    pub y: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct TerrainVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

#[derive(Debug)]
pub struct TerrainMesh {
    pub vertices: Vec<TerrainVertex>,
    pub indices: Vec<u32>,
}

#[derive(Debug)]
pub struct TerrainTile {
    pub key: TileKey,
    pub center: [f32; 3],
    pub mesh: TerrainMesh,
}

pub struct PlanetTerrain {
    pub radius: f32,
    pub tile_resolution: u32,
    pub tile_count_per_face: u32,
    pub load_distance: f32,
    pub unload_distance: f32,
    pub seed: u32,
    pub loaded_tiles: HashMap<TileKey, TerrainTile>,
}

impl PlanetTerrain {
    pub fn new(radius: f32, tile_resolution: u32, seed: u32) -> Self {
        Self {
            radius,
            tile_resolution: tile_resolution.max(2),
            tile_count_per_face: 4,
            load_distance: radius * 3.5,
            unload_distance: radius * 4.5,
            seed,
            loaded_tiles: HashMap::new(),
        }
    }

    pub fn update_streaming(&mut self, camera_position: [f32; 3]) {
        let mut required = HashSet::new();
        let tile_span = 2.0 / self.tile_count_per_face as f32;
        for face in 0..6 {
            for y in 0..self.tile_count_per_face {
                for x in 0..self.tile_count_per_face {
                    let center = cube_face_point(
                        face,
                        -1.0 + (x as f32 + 0.5) * tile_span,
                        -1.0 + (y as f32 + 0.5) * tile_span,
                    );
                    let world_center = scale(center, self.radius);
                    if distance(camera_position, world_center) <= self.load_distance {
                        required.insert(TileKey { face, x, y });
                    }
                }
            }
        }

        for key in required.iter().copied() {
            if !self.loaded_tiles.contains_key(&key) {
                let tile = self.generate_tile(key);
                self.loaded_tiles.insert(key, tile);
            }
        }

        self.loaded_tiles.retain(|key, tile| {
            required.contains(key) || distance(camera_position, tile.center) <= self.unload_distance
        });
    }

    pub fn loaded_tile_count(&self) -> usize {
        self.loaded_tiles.len()
    }

    fn generate_tile(&self, key: TileKey) -> TerrainTile {
        let vertex_side = self.tile_resolution + 1;
        let mut vertices = Vec::with_capacity((vertex_side * vertex_side) as usize);
        let mut indices =
            Vec::with_capacity((self.tile_resolution * self.tile_resolution * 6) as usize);
        let tile_span = 2.0 / self.tile_count_per_face as f32;
        let min_u = -1.0 + key.x as f32 * tile_span;
        let min_v = -1.0 + key.y as f32 * tile_span;

        for y in 0..=self.tile_resolution {
            for x in 0..=self.tile_resolution {
                let u = min_u + x as f32 / self.tile_resolution as f32 * tile_span;
                let v = min_v + y as f32 / self.tile_resolution as f32 * tile_span;
                let direction = normalize(cube_face_point(key.face, u, v));
                let height = noise::simplex(
                    self.seed,
                    direction[0] * 3.0 + key.face as f32,
                    direction[2] * 3.0,
                ) * self.radius
                    * 0.08;
                let position = scale(direction, self.radius + height);
                vertices.push(TerrainVertex {
                    position,
                    normal: direction,
                });
            }
        }

        for y in 0..self.tile_resolution {
            for x in 0..self.tile_resolution {
                let row = vertex_side * y;
                let next_row = vertex_side * (y + 1);
                indices.extend_from_slice(&[
                    row + x,
                    next_row + x,
                    row + x + 1,
                    row + x + 1,
                    next_row + x,
                    next_row + x + 1,
                ]);
            }
        }

        let center = scale(
            normalize(cube_face_point(
                key.face,
                min_u + tile_span * 0.5,
                min_v + tile_span * 0.5,
            )),
            self.radius,
        );
        TerrainTile {
            key,
            center,
            mesh: TerrainMesh { vertices, indices },
        }
    }
}

fn cube_face_point(face: u8, u: f32, v: f32) -> [f32; 3] {
    match face {
        0 => [1.0, v, -u],
        1 => [-1.0, v, u],
        2 => [u, 1.0, -v],
        3 => [u, -1.0, v],
        4 => [u, v, 1.0],
        _ => [-u, v, -1.0],
    }
}

fn normalize(value: [f32; 3]) -> [f32; 3] {
    let length = (value[0] * value[0] + value[1] * value[1] + value[2] * value[2]).sqrt();
    [value[0] / length, value[1] / length, value[2] / length]
}

fn scale(value: [f32; 3], factor: f32) -> [f32; 3] {
    [value[0] * factor, value[1] * factor, value[2] * factor]
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::PlanetTerrain;

    #[test]
    fn streaming_generates_tiles_and_keeps_meshes_consistent() {
        let mut planet = PlanetTerrain::new(10.0, 4, 42);
        planet.update_streaming([0.0, 0.0, 30.0]);

        assert!(!planet.loaded_tiles.is_empty());
        let tile = planet.loaded_tiles.values().next().unwrap();
        assert_eq!(tile.mesh.vertices.len(), 25);
        assert_eq!(tile.mesh.indices.len(), 96);
    }
}

#[derive(Clone, Copy)]
pub struct Transform {
    pub position: [f32; 3],
    pub scale: f32,
    pub rotation: f32,
}

#[derive(Clone, Copy)]
pub struct Mesh {
    pub vertex_count: u32,
    pub color: [f32; 3],
}

pub struct Entity {
    pub transform: Transform,
    pub mesh: Mesh,
    pub visible: bool,
}

pub struct Camera {
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    move_speed: f32,
    look_sensitivity: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CameraPushConstants {
    pub camera_position_yaw: [f32; 4],
    pub camera_pitch_entity_scale: [f32; 4],
    pub entity_position_rotation: [f32; 4],
    pub entity_color: [f32; 4],
}

pub struct Scene {
    pub camera: Camera,
    pub entities: Vec<Entity>,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 3.0],
            yaw: 0.0,
            pitch: 0.0,
            move_speed: 3.0,
            look_sensitivity: 0.0025,
        }
    }
}

impl Camera {
    pub fn update(
        &mut self,
        forward_pressed: bool,
        backward_pressed: bool,
        left_pressed: bool,
        right_pressed: bool,
        mouse_delta: &mut (f32, f32),
        delta_time: f32,
    ) {
        self.yaw -= mouse_delta.0 * self.look_sensitivity;
        self.pitch = (self.pitch - mouse_delta.1 * self.look_sensitivity)
            .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
        *mouse_delta = (0.0, 0.0);

        let forward = [self.yaw.sin(), 0.0, -self.yaw.cos()];
        let right = [self.yaw.cos(), 0.0, self.yaw.sin()];
        let mut movement = [0.0, 0.0, 0.0];
        if forward_pressed {
            movement[0] += forward[0];
            movement[2] += forward[2];
        }
        if backward_pressed {
            movement[0] -= forward[0];
            movement[2] -= forward[2];
        }
        if right_pressed {
            movement[0] += right[0];
            movement[2] += right[2];
        }
        if left_pressed {
            movement[0] -= right[0];
            movement[2] -= right[2];
        }

        let length = (movement[0] * movement[0] + movement[2] * movement[2]).sqrt();
        if length > 0.0 {
            let distance = self.move_speed * delta_time / length;
            self.position[0] += movement[0] * distance;
            self.position[2] += movement[2] * distance;
        }
    }
}

impl Scene {
    pub fn demo() -> Self {
        let triangle = Mesh {
            vertex_count: 3,
            color: [1.0, 0.3, 0.2],
        };
        Self {
            camera: Camera::default(),
            entities: vec![
                Entity {
                    transform: Transform {
                        position: [-1.3, 0.0, 0.0],
                        scale: 0.8,
                        rotation: 0.0,
                    },
                    mesh: triangle,
                    visible: true,
                },
                Entity {
                    transform: Transform {
                        position: [1.3, 0.0, 0.0],
                        scale: 0.8,
                        rotation: 0.0,
                    },
                    mesh: Mesh {
                        color: [0.2, 0.8, 1.0],
                        ..triangle
                    },
                    visible: true,
                },
            ],
        }
    }

    pub fn visible_entities(&self) -> impl Iterator<Item = &Entity> {
        self.entities.iter().filter(|entity| entity.visible)
    }

    pub fn push_constants(&self, entity: &Entity) -> CameraPushConstants {
        CameraPushConstants {
            camera_position_yaw: [
                self.camera.position[0],
                self.camera.position[1],
                self.camera.position[2],
                self.camera.yaw,
            ],
            camera_pitch_entity_scale: [self.camera.pitch, entity.transform.scale, 0.0, 0.0],
            entity_position_rotation: [
                entity.transform.position[0],
                entity.transform.position[1],
                entity.transform.position[2],
                entity.transform.rotation,
            ],
            entity_color: [
                entity.mesh.color[0],
                entity.mesh.color[1],
                entity.mesh.color[2],
                1.0,
            ],
        }
    }
}

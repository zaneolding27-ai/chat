const GRADIENTS: [[f32; 2]; 8] = [
    [1.0, 0.0],
    [-1.0, 0.0],
    [0.0, 1.0],
    [0.0, -1.0],
    [0.70710677, 0.70710677],
    [-0.70710677, 0.70710677],
    [0.70710677, -0.70710677],
    [-0.70710677, -0.70710677],
];

/// Returns deterministic 2D simplex noise in the inclusive range [-1, 1].
pub fn simplex(seed: u32, x: f32, y: f32) -> f32 {
    const F2: f32 = 0.3660254;
    const G2: f32 = 0.21132487;

    let skew = (x + y) * F2;
    let cell_x = (x + skew).floor() as i32;
    let cell_y = (y + skew).floor() as i32;
    let unskew = (cell_x + cell_y) as f32 * G2;
    let origin_x = cell_x as f32 - unskew;
    let origin_y = cell_y as f32 - unskew;
    let local_x = x - origin_x;
    let local_y = y - origin_y;

    let (offset_x, offset_y) = if local_x > local_y { (1, 0) } else { (0, 1) };
    let corners = [
        (local_x, local_y, 0),
        (
            local_x - offset_x as f32 + G2,
            local_y - offset_y as f32 + G2,
            1,
        ),
        (local_x - 1.0 + 2.0 * G2, local_y - 1.0 + 2.0 * G2, 2),
    ];

    let mut value = 0.0;
    for (corner_x, corner_y, corner) in corners {
        let attenuation = 0.5 - corner_x * corner_x - corner_y * corner_y;
        if attenuation > 0.0 {
            let gradient = GRADIENTS[hash(
                seed,
                cell_x
                    + if corner > 0 {
                        if corner == 1 {
                            offset_x
                        } else {
                            1
                        }
                    } else {
                        0
                    },
                cell_y
                    + if corner > 0 {
                        if corner == 1 {
                            offset_y
                        } else {
                            1
                        }
                    } else {
                        0
                    },
            ) as usize
                % GRADIENTS.len()];
            let contribution = gradient[0] * corner_x + gradient[1] * corner_y;
            value += attenuation.powi(4) * contribution;
        }
    }

    (value * 70.0).clamp(-1.0, 1.0)
}

fn hash(seed: u32, x: i32, y: i32) -> u32 {
    let mut value = seed
        .wrapping_add((x as u32).wrapping_mul(0x9E37_79B9))
        .wrapping_add((y as u32).wrapping_mul(0x85EB_CA6B));
    value ^= value >> 16;
    value = value.wrapping_mul(0x7FEB_352D);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846C_A68B);
    value ^ (value >> 16)
}

#[cfg(test)]
mod tests {
    use super::simplex;

    #[test]
    fn same_seed_and_coordinates_are_repeatable() {
        let first = simplex(42, 12.5, -3.25);
        let second = simplex(42, 12.5, -3.25);

        assert_eq!(first, second);
        assert!((-1.0..=1.0).contains(&first));
    }

    #[test]
    fn different_seed_changes_the_noise_field() {
        assert_ne!(simplex(42, 12.5, -3.25), simplex(43, 12.5, -3.25));
    }
}

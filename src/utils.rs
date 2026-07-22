use nalgebra::Vector3;

pub fn laplacian_smooth(vertices: &mut [f32], indices: &[u32], iterations: usize, lambda: f32) {
    for _ in 0..iterations {
        let mut new_positions = vec![0.0; vertices.len()];
        let mut counts = vec![0; vertices.len() / 3];

        for face in indices.chunks(3) {
            for i in 0..3 {
                let v_idx = face[i] as usize;
                let v1 = face[(i + 1) % 3] as usize;
                let v2 = face[(i + 2) % 3] as usize;

                for k in 0..3 {
                    new_positions[v_idx * 3 + k] += vertices[v1 * 3 + k] + vertices[v2 * 3 + k];
                }
                counts[v_idx] += 2;
            }
        }

        for i in 0..vertices.len() / 3 {
            if counts[i] > 0 {
                for k in 0..3 {
                    let avg = new_positions[i * 3 + k] / counts[i] as f32;
                    vertices[i * 3 + k] += lambda * (avg - vertices[i * 3 + k]);
                }
            }
        }
    }
}

pub fn is_valid_face(
    p0: &[f32; 3],
    p1: &[f32; 3],
    p2: &[f32; 3],
    threshold: f32,
    safe_z_diff: f32,
) -> bool {
    let min_z = p0[2].min(p1[2]).min(p2[2]);
    let max_z = p0[2].max(p1[2]).max(p2[2]);

    if (max_z - min_z) < safe_z_diff {
        return true;
    }

    let v0 = Vector3::new(p0[0], p0[1], p0[2]);
    let v1 = Vector3::new(p1[0], p1[1], p1[2]);
    let v2 = Vector3::new(p2[0], p2[1], p2[2]);

    let e1 = v1 - v0;
    let e2 = v2 - v0;

    let normal = e1.cross(&e2);
    let norm_len = normal.norm();
    if norm_len < 1e-6 {
        return false;
    }
    let normal = normal / norm_len;

    let center = (v0 + v1 + v2) / 3.0;
    let center_len = center.norm();
    if center_len < 1e-6 {
        return false;
    }
    let center = center / center_len;

    let cos_theta = normal.dot(&center).abs();
    let clamped_cos = cos_theta.clamp(0.0, 1.0);
    let face_ray_angle_deg = clamped_cos.asin().to_degrees();

    face_ray_angle_deg > threshold
}

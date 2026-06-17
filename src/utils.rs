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

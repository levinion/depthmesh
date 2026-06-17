use image::{EncodableLayout, GrayImage, RgbImage};
use meshopt::{
    SimplifyOptions, VertexDataAdapter, generate_vertex_remap, remap_index_buffer,
    remap_vertex_buffer, simplify,
};
use std::fs::File;
use std::io::{BufWriter, Write};
pub struct Mesh {
    vertices: Vec<f32>,
    indices: Vec<u32>,
    texcoords: Vec<f32>,
    normals: Vec<f32>,
}

impl Mesh {
    pub fn new(img: GrayImage, scale: f32, normal: Option<RgbImage>) -> Self {
        let (width, height) = img.dimensions();

        let mut vertices: Vec<f32> = Vec::with_capacity((width * height * 3) as usize);
        let mut indices: Vec<u32> = Vec::new();
        let mut texcoords = Vec::with_capacity((width * height * 2) as usize);
        let mut normals = Vec::with_capacity((width * height * 3) as usize);

        let mut valid = vec![false; (width * height) as usize];

        for y in 0..height {
            for x in 0..width {
                let pixel = img.get_pixel(x, y);
                // calculate vertices
                let depth_value = pixel.0[0];

                let idx = (y * width + x) as usize;
                valid[idx] = depth_value > 0;

                let coord_x = x as f32;
                let coord_y = (height - 1 - y) as f32;
                let coord_z = depth_value as f32 * scale;

                vertices.push(coord_x);
                vertices.push(coord_y);
                vertices.push(coord_z);

                // calculate texcoords
                let u = x as f32 / (width - 1) as f32;
                let v = 1.0 - y as f32 / (height - 1) as f32;

                texcoords.push(u);
                texcoords.push(v);

                // calculate normals
                if let Some(ref normal) = normal {
                    let npixel = normal.get_pixel(x, y);

                    let nx = npixel[0] as f32 / 255.0 * 2.0 - 1.0;
                    let ny = npixel[1] as f32 / 255.0 * 2.0 - 1.0;
                    let nz = npixel[2] as f32 / 255.0 * 2.0 - 1.0;

                    let len = (nx * nx + ny * ny + nz * nz).sqrt();

                    normals.push(nx / len);
                    normals.push(ny / len);
                    normals.push(nz / len);
                }
            }
        }

        for y in 0..(height - 1) {
            for x in 0..(width - 1) {
                let v0 = (y * width + x) as usize;
                let v1 = (y * width + (x + 1)) as usize;
                let v2 = ((y + 1) * width + x) as usize;
                let v3 = ((y + 1) * width + (x + 1)) as usize;

                if valid[v0] && valid[v1] && valid[v2] {
                    indices.push(v0 as u32);
                    indices.push(v2 as u32);
                    indices.push(v1 as u32);
                }

                if valid[v1] && valid[v2] && valid[v3] {
                    indices.push(v1 as u32);
                    indices.push(v2 as u32);
                    indices.push(v3 as u32);
                }
            }
        }

        let mut mesh = Self {
            vertices,
            indices,
            texcoords,
            normals,
        };

        mesh.normalize();
        mesh
    }

    pub fn compute_normals(&mut self) {
        let vertex_count = self.vertices.len() / 3;

        self.normals = vec![0.0; vertex_count * 3];

        for tri in self.indices.chunks_exact(3) {
            let i0 = tri[0] as usize;
            let i1 = tri[1] as usize;
            let i2 = tri[2] as usize;

            let p0 = [
                self.vertices[i0 * 3],
                self.vertices[i0 * 3 + 1],
                self.vertices[i0 * 3 + 2],
            ];

            let p1 = [
                self.vertices[i1 * 3],
                self.vertices[i1 * 3 + 1],
                self.vertices[i1 * 3 + 2],
            ];

            let p2 = [
                self.vertices[i2 * 3],
                self.vertices[i2 * 3 + 1],
                self.vertices[i2 * 3 + 2],
            ];

            let e1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];

            let e2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];

            let n = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];

            for i in [i0, i1, i2] {
                self.normals[i * 3] += n[0];
                self.normals[i * 3 + 1] += n[1];
                self.normals[i * 3 + 2] += n[2];
            }
        }

        for n in self.normals.chunks_exact_mut(3) {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();

            if len > 1e-6 {
                n[0] /= len;
                n[1] /= len;
                n[2] /= len;
            }
        }
    }

    fn normalize(&mut self) {
        if self.vertices.is_empty() {
            return;
        }

        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let mut min_z = f32::INFINITY;
        let mut max_z = f32::NEG_INFINITY;

        for chunk in self.vertices.chunks_exact(3) {
            let x = chunk[0];
            let y = chunk[1];
            let z = chunk[2];

            if x < min_x {
                min_x = x;
            }
            if x > max_x {
                max_x = x;
            }
            if y < min_y {
                min_y = y;
            }
            if y > max_y {
                max_y = y;
            }
            if z < min_z {
                min_z = z;
            }
            if z > max_z {
                max_z = z;
            }
        }

        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;
        let center_z = (min_z + max_z) / 2.0;

        let span_x = max_x - min_x;
        let span_y = max_y - min_y;
        let span_z = max_z - min_z;

        let max_span = span_x.max(span_y).max(span_z);
        if max_span == 0.0 {
            return;
        }

        let scale = 2.0 / max_span;

        for chunk in self.vertices.chunks_exact_mut(3) {
            chunk[0] = (chunk[0] - center_x) * scale;
            chunk[1] = (chunk[1] - center_y) * scale;
            chunk[2] = (chunk[2] - center_z) * scale;
        }
    }

    pub fn optimize(&mut self, reduction: f32, error: f32) {
        let target_count = (self.indices.len() as f32 * reduction) as usize;

        let vertex_data = VertexDataAdapter::new(self.vertices.as_bytes(), 12, 0).unwrap();

        let indices = simplify(
            &self.indices,
            &vertex_data,
            target_count,
            error,
            SimplifyOptions::empty(),
            None,
        );

        let (vs, _) = self.vertices.as_chunks::<3>();
        let (vertex_count, remap_table) = generate_vertex_remap(vs, Some(&indices));

        // re-calculate vertices
        self.vertices = remap_vertex_buffer(vs, vertex_count, &remap_table).into_flattened();

        // re-calculate uvs
        let (uvs, _) = self.texcoords.as_chunks::<2>();
        self.texcoords = remap_vertex_buffer(uvs, vertex_count, &remap_table).into_flattened();

        // re-calculate normals
        let (normals, _) = self.normals.as_chunks::<3>();
        self.normals = remap_vertex_buffer(normals, vertex_count, &remap_table).into_flattened();

        self.indices = remap_index_buffer(Some(&indices), vertex_count, &remap_table);
    }

    pub fn smooth(&mut self, iterations: usize, lambda: f32) {
        crate::utils::laplacian_smooth(&mut self.vertices, &self.indices, iterations, lambda);
    }

    pub fn write(&mut self, path: &str) {
        let file = File::create(path).unwrap();
        let mut writer = BufWriter::new(file);

        for v in self.vertices.chunks(3) {
            writeln!(writer, "v {:.4} {:.4} {:.4}", v[0], v[1], v[2]).unwrap();
        }

        for vt in self.texcoords.chunks_exact(2) {
            writeln!(writer, "vt {:.4} {:.4}", vt[0], vt[1]).unwrap();
        }

        if self.normals.is_empty() {
            self.compute_normals();
        }
        for vn in self.normals.chunks_exact(3) {
            writeln!(writer, "vn {:.4} {:.4} {:.4}", vn[0], vn[1], vn[2]).unwrap();
        }

        for f in self.indices.chunks_exact(3) {
            writeln!(
                writer,
                "f {0}/{0}/{0} {1}/{1}/{1} {2}/{2}/{2}",
                f[0] + 1,
                f[1] + 1,
                f[2] + 1
            )
            .unwrap();
        }

        writer.flush().unwrap();
    }
}

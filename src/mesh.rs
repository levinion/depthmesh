use anyhow::{Context, Result, bail};
use image::{EncodableLayout, ImageBuffer, Rgb};
use meshopt::{
    SimplifyOptions, VertexDataAdapter, generate_vertex_remap, remap_index_buffer,
    remap_vertex_buffer, simplify,
};
use nalgebra::{Matrix3, Matrix4};
use std::fs::File;
use std::io::{BufWriter, Write};
pub struct Mesh {
    vertices: Vec<f32>,
    indices: Vec<u32>,
    texcoords: Vec<f32>,
    normals: Vec<f32>,
    offset: f32,
}

type Rgb32FImage = ImageBuffer<Rgb<f32>, Vec<f32>>;

impl Mesh {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        depth: Rgb32FImage,
        threshold: f32,
        intrinsic: Matrix3<f32>,
        normal: Option<Rgb32FImage>,
        scale: f32,
        reverse_z: bool,
        distance: bool,
        offset: f32,
    ) -> Result<Self> {
        let (width, height) = depth.dimensions();

        let fx = intrinsic[(0, 0)];
        let fy = intrinsic[(1, 1)];
        let cx = intrinsic[(0, 2)];
        let cy = intrinsic[(1, 2)];

        let mut vertices: Vec<f32> = Vec::with_capacity((width * height * 3) as usize);
        let mut indices: Vec<u32> = Vec::new();
        let mut texcoords = Vec::with_capacity((width * height * 2) as usize);
        let mut normals = Vec::with_capacity((width * height * 3) as usize);

        let mut valid = vec![0u32; (width * height) as usize];
        let mut index = 1;

        for y in 0..height {
            for x in 0..width {
                let pixel = depth.get_pixel(x, y);
                // calculate vertices
                let depth_value = pixel.0[0] * scale;

                let is_valid = depth_value.is_finite();

                if is_valid {
                    let idx = (y * width + x) as usize;
                    valid[idx] = index;
                    index += 1;
                    let (coord_x, coord_y, coord_z) = {
                        let rx = (x as f32 - cx) / fx;
                        let ry = (cy - y as f32) / fy;
                        let z = if distance {
                            let ray_len = (rx * rx + ry * ry + 1.0).sqrt();
                            depth_value / ray_len
                        } else {
                            depth_value
                        };
                        let coord_x = rx * z;
                        let coord_y = ry * z;
                        let coord_z = if reverse_z { z } else { -z };
                        (coord_x, coord_y, coord_z)
                    };

                    vertices.push(coord_x);
                    vertices.push(coord_y);
                    vertices.push(coord_z);

                    // calculate texcoords
                    let u = x as f32 / width as f32;
                    let v = 1.0 - y as f32 / height as f32;

                    texcoords.push(u);
                    texcoords.push(v);

                    // calculate normals
                    if let Some(ref normal) = normal {
                        let npixel = normal.get_pixel(x, y);

                        let nx = npixel[0];
                        let ny = -npixel[1];
                        let nz = if reverse_z { npixel[2] } else { -npixel[2] };

                        let length = (nx * nx + ny * ny + nz * nz).sqrt();

                        normals.push(nx / length);
                        normals.push(ny / length);
                        normals.push(nz / length);
                    }
                }
            }
        }

        let min_depth = vertices
            .chunks_exact(3)
            .map(|chuck| chuck[2])
            .fold(f32::INFINITY, f32::min);

        let max_depth = vertices
            .chunks_exact(3)
            .map(|chuck| chuck[2])
            .fold(f32::NEG_INFINITY, f32::max);

        let max_depth_diff = max_depth - min_depth;

        for y in 0..(height - 1) {
            for x in 0..(width - 1) {
                let v0 = (y * width + x) as usize;
                let v1 = (y * width + (x + 1)) as usize;
                let v2 = ((y + 1) * width + x) as usize;
                let v3 = ((y + 1) * width + (x + 1)) as usize;

                let d0 = depth.get_pixel(x, y)[0] * scale;
                let d1 = depth.get_pixel(x + 1, y)[0] * scale;
                let d2 = depth.get_pixel(x, y + 1)[0] * scale;
                let d3 = depth.get_pixel(x + 1, y + 1)[0] * scale;

                let (z0, z1, z2, z3) = if distance {
                    let rx = (x as f32 - cx) / fx;
                    let ry = (cy - y as f32) / fy;
                    let ray_len = (rx * rx + ry * ry + 1.0).sqrt();
                    (d0 / ray_len, d1 / ray_len, d2 / ray_len, d3 / ray_len)
                } else {
                    (d0, d1, d2, d3)
                };

                if valid[v0] > 0
                    && valid[v1] > 0
                    && valid[v2] > 0
                    && (threshold <= 0.
                        || ((crate::utils::max_depth_diff(&[z0, z1, z2]) / max_depth_diff)
                            < threshold))
                {
                    indices.push(valid[v0] - 1);
                    indices.push(valid[v1] - 1);
                    indices.push(valid[v2] - 1);
                }

                if valid[v1] > 0
                    && valid[v2] > 0
                    && valid[v3] > 0
                    && (threshold <= 0.
                        || ((crate::utils::max_depth_diff(&[z1, z2, z3]) / max_depth_diff)
                            < threshold))
                {
                    indices.push(valid[v1] - 1);
                    indices.push(valid[v2] - 1);
                    indices.push(valid[v3] - 1);
                }
            }
        }

        let mesh = Self {
            vertices,
            indices,
            texcoords,
            normals,
            offset,
        };

        Ok(mesh)
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

    pub fn optimize(&mut self, reduction: f32, error: f32) -> Result<()> {
        let target_count = (self.indices.len() as f32 * reduction) as usize;

        let vertex_data = VertexDataAdapter::new(self.vertices.as_bytes(), 12, 0)?;

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

        Ok(())
    }

    pub fn smooth(&mut self, iterations: usize, lambda: f32) {
        crate::utils::laplacian_smooth(&mut self.vertices, &self.indices, iterations, lambda);
    }

    pub fn write(&mut self, path: &str) -> Result<()> {
        let vertices = self.vertices.chunks_exact(3);
        let texcoords = self.texcoords.chunks_exact(2);
        let normals = self.normals.chunks_exact(3);

        if vertices.len() != texcoords.len() || vertices.len() != normals.len() {
            bail!("vertices, texcoords, normals not matched, the mesh will be wrong!");
        }

        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        for v in self.vertices.chunks_exact(3) {
            writeln!(writer, "v {:.6} {:.6} {:.6}", v[0], v[1], v[2])?;
        }

        for vt in self.texcoords.chunks_exact(2) {
            writeln!(writer, "vt {:.6} {:.6}", vt[0], vt[1])?;
        }

        if self.normals.is_empty() {
            self.compute_normals();
        }
        for vn in self.normals.chunks_exact(3) {
            writeln!(writer, "vn {:.6} {:.6} {:.6}", vn[0], vn[1], vn[2])?;
        }

        for i in self.indices.chunks_exact(3) {
            writeln!(
                writer,
                "f {0}/{0}/{0} {1}/{1}/{1} {2}/{2}/{2}",
                i[0] + 1,
                i[1] + 1,
                i[2] + 1
            )?;
        }

        writer.flush()?;
        Ok(())
    }

    pub fn transform(&mut self, src: Matrix4<f32>, target: Matrix4<f32>) -> Result<()> {
        // S = T_world_to_cam * S_cam_to_world * S
        let t = target
            .try_inverse()
            .map(|inv| inv * src)
            .context("cannot inverse target pose matrix")?;

        let translation = t.fixed_view::<3, 1>(0, 3).into_owned();
        let rotation = t.fixed_view::<3, 3>(0, 0).into_owned();

        for v in self.vertices.chunks_exact_mut(3) {
            let pos = nalgebra::Vector3::new(v[0], v[1], v[2]);
            let transformed = rotation * pos + translation;
            v[0] = transformed.x;
            v[1] = transformed.y;
            v[2] = transformed.z - self.offset;
        }

        if !self.normals.is_empty() {
            for n in self.normals.chunks_exact_mut(3) {
                let nn = (rotation * nalgebra::Vector3::new(n[0], n[1], n[2])).normalize();
                n[0] = nn.x;
                n[1] = nn.y;
                n[2] = nn.z;
            }
        }

        Ok(())
    }
}

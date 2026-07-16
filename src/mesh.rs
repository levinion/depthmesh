use anyhow::{Context, Result, bail};
use image::{EncodableLayout, ImageBuffer, Rgb};
use meshopt::{
    SimplifyOptions, VertexDataAdapter, generate_vertex_remap, remap_index_buffer,
    remap_vertex_buffer, simplify,
};
use nalgebra::{Matrix3, Matrix4};
use std::fs::File;
use std::io::{BufWriter, Write};

#[derive(Default)]
pub struct Mesh {
    vertices: Vec<f32>,
    indices: Vec<u32>,
    texcoords: Vec<f32>,
    attributes: Vec<(String, Vec<f32>)>,
}

type Rgb32FImage = ImageBuffer<Rgb<f32>, Vec<f32>>;

impl Mesh {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        depth: Rgb32FImage,
        threshold: f32,
        intrinsic: Matrix3<f32>,
        scale: f32,
        reverse_z: bool,
        distance: bool,
    ) -> Result<Self> {
        let (width, height) = depth.dimensions();

        let fx = intrinsic[(0, 0)];
        let fy = intrinsic[(1, 1)];
        let cx = intrinsic[(0, 2)];
        let cy = intrinsic[(1, 2)];

        let mut vertices: Vec<f32> = Vec::with_capacity((width * height * 3) as usize);
        let mut indices: Vec<u32> = Vec::new();
        let mut texcoords = Vec::with_capacity((width * height * 2) as usize);

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
            ..Default::default()
        };

        Ok(mesh)
    }

    pub fn sample(&mut self, name: &str, img: &Rgb32FImage, channel: usize) {
        let vertex_count = self.vertices.len() / 3;
        let (width, height) = img.dimensions();

        if channel > 2 || width == 0 || height == 0 || self.texcoords.len() / 2 != vertex_count {
            return;
        }

        let raw_pixels = img.as_raw();
        let mut sampled_channel = Vec::with_capacity(vertex_count);

        for uv in self.texcoords.chunks_exact(2) {
            let u = uv[0].clamp(0.0, 1.0);
            let v = (1.0 - uv[1]).clamp(0.0, 1.0);

            let x = ((u * (width - 1) as f32).round() as u32).min(width - 1);
            let y = ((v * (height - 1) as f32).round() as u32).min(height - 1);

            let pixel_idx = ((y * width + x) as usize) * 3;

            let val = raw_pixels[pixel_idx + channel];

            sampled_channel.push(val);
        }

        self.attributes.push((name.to_owned(), sampled_channel));
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
        self.indices = remap_index_buffer(Some(&indices), vertex_count, &remap_table);

        Ok(())
    }

    pub fn smooth(&mut self, iterations: usize, lambda: f32) {
        crate::utils::laplacian_smooth(&mut self.vertices, &self.indices, iterations, lambda);
    }

    pub fn transform(
        &mut self,
        src: Matrix4<f32>,
        target: Matrix4<f32>,
        offset: f32,
    ) -> Result<()> {
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
            v[2] = transformed.z - offset;
        }

        Ok(())
    }

    pub fn write(&mut self, path: &str) -> Result<()> {
        let num_vertices = self.vertices.len() / 3;
        let num_faces = self.indices.len() / 3;

        let has_texcoords = !self.texcoords.is_empty();
        if has_texcoords && self.texcoords.len() / 2 != num_vertices {
            bail!("texcoords count does not match vertices!");
        }

        let extra_keys: Vec<String> = self
            .attributes
            .iter()
            .filter(|(_, vec)| vec.len() == num_vertices)
            .map(|(k, _)| k.clone())
            .collect();

        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        writeln!(writer, "ply")?;
        writeln!(writer, "format ascii 1.0")?;
        writeln!(writer, "comment Created by depthmesh")?;

        writeln!(writer, "element vertex {}", num_vertices)?;

        writeln!(writer, "property float x")?;
        writeln!(writer, "property float y")?;
        writeln!(writer, "property float z")?;

        if has_texcoords {
            writeln!(writer, "property float s")?;
            writeln!(writer, "property float t")?;
        }

        for key in &extra_keys {
            writeln!(writer, "property float {}", key)?;
        }

        writeln!(writer, "element face {}", num_faces)?;
        writeln!(writer, "property list uchar int vertex_indices")?;
        writeln!(writer, "end_header")?;

        let extra_cols: Vec<&[f32]> = extra_keys
            .iter()
            .map(|k| {
                self.attributes
                    .iter()
                    .find(|(key, _)| key == k)
                    .map(|(_, vec)| vec.as_slice())
                    .unwrap()
            })
            .collect();

        for i in 0..num_vertices {
            write!(
                writer,
                "{:.6} {:.6} {:.6}",
                self.vertices[i * 3],
                self.vertices[i * 3 + 1],
                self.vertices[i * 3 + 2]
            )?;

            if has_texcoords {
                write!(
                    writer,
                    " {:.6} {:.6}",
                    self.texcoords[i * 2],
                    1.0 - self.texcoords[i * 2 + 1]
                )?;
            }

            for col in &extra_cols {
                write!(writer, " {:.6}", col[i])?;
            }

            writeln!(writer)?;
        }

        for face in self.indices.chunks_exact(3) {
            writeln!(writer, "3 {} {} {}", face[0], face[1], face[2])?;
        }

        writer.flush()?;
        Ok(())
    }
}

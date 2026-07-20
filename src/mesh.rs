use anyhow::{Context, Result, bail};
use image::{EncodableLayout, ImageBuffer, Rgb};
use meshopt::{
    SimplifyOptions, VertexDataAdapter, generate_vertex_remap, remap_index_buffer,
    remap_vertex_buffer, simplify,
};
use nalgebra::{Matrix3, Matrix4, Vector3};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};

use crate::utils;

#[derive(Default)]
pub struct Mesh {
    vertices: Vec<f32>,
    indices: Vec<u32>,
    texcoords: Vec<f32>,
    attributes: Vec<(String, Vec<f32>)>,
    normals: Option<Vec<f32>>,
}

type Rgb32FImage = ImageBuffer<Rgb<f32>, Vec<f32>>;

impl Mesh {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mut depth: Rgb32FImage,
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

        for y in 0..height {
            for x in 0..width {
                let pixel = depth.get_pixel(x, y);

                let depth_value = pixel.0[0] * scale;
                if depth_value.is_infinite() || depth_value < 1e-6 {
                    continue;
                }
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

                let pixel = depth.get_pixel_mut(x, y);

                pixel.0 = [coord_x, coord_y, coord_z]
            }
        }

        let mut pixel2index = HashMap::new();
        let mut texcoords = Vec::with_capacity((width * height * 2) as usize);

        for y in 0..height {
            for x in 0..width {
                let pixel = depth.get_pixel(x, y);
                // calculate vertices
                let depth_value = pixel.0[0] * scale;

                if depth_value.is_infinite() {
                    continue;
                }

                let current_idx = (vertices.len() / 3) as u32;
                vertices.extend(&pixel.0);
                pixel2index.insert((y * width + x) as usize, current_idx);

                // calculate texcoords
                let u = x as f32 / width as f32;
                let v = 1.0 - y as f32 / height as f32;

                texcoords.push(u);
                texcoords.push(v);
            }
        }

        let mut indices: Vec<u32> = Vec::new();

        for y in 0..(height - 1) {
            for x in 0..(width - 1) {
                let idx_0 = pixel2index.get(&((y * width + x) as usize));
                let idx_1 = pixel2index.get(&((y * width + (x + 1)) as usize));
                let idx_2 = pixel2index.get(&(((y + 1) * width + x) as usize));
                let idx_3 = pixel2index.get(&(((y + 1) * width + (x + 1)) as usize));

                let p0 = depth.get_pixel(x, y).0;
                let p1 = depth.get_pixel(x + 1, y).0;
                let p2 = depth.get_pixel(x, y + 1).0;
                let p3 = depth.get_pixel(x + 1, y + 1).0;

                if let (Some(&i0), Some(&i1), Some(&i2)) = (idx_0, idx_1, idx_2)
                    && utils::is_valid_face(&p0, &p1, &p2, threshold)
                {
                    indices.push(i0);
                    indices.push(i1);
                    indices.push(i2);
                }

                if let (Some(&i1), Some(&i2), Some(&i3)) = (idx_1, idx_2, idx_3)
                    && utils::is_valid_face(&p1, &p3, &p2, threshold)
                {
                    indices.push(i1);
                    indices.push(i3);
                    indices.push(i2);
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

    pub fn compute_normal(&mut self, img: Option<Rgb32FImage>) {
        if let Some(img) = img {
            self.sample("nx", &img, 0);
            self.sample("ny", &img, 1);
            self.sample("nz", &img, 2);
        } else {
            let mut normals = vec![0.0f32; self.vertices.len()];

            for chunk in self.indices.chunks_exact(3) {
                let i0 = chunk[0] as usize;
                let i1 = chunk[1] as usize;
                let i2 = chunk[2] as usize;

                let v0 = Vector3::new(
                    self.vertices[i0 * 3],
                    self.vertices[i0 * 3 + 1],
                    self.vertices[i0 * 3 + 2],
                );
                let v1 = Vector3::new(
                    self.vertices[i1 * 3],
                    self.vertices[i1 * 3 + 1],
                    self.vertices[i1 * 3 + 2],
                );
                let v2 = Vector3::new(
                    self.vertices[i2 * 3],
                    self.vertices[i2 * 3 + 1],
                    self.vertices[i2 * 3 + 2],
                );

                let edge1 = v1 - v0;
                let edge2 = v2 - v0;

                let face_normal = edge2.cross(&edge1);

                for &idx in &[i0, i1, i2] {
                    normals[idx * 3] += face_normal.x;
                    normals[idx * 3 + 1] += face_normal.y;
                    normals[idx * 3 + 2] += face_normal.z;
                }
            }

            for i in 0..self.vertices.len() / 3 {
                let mut n = Vector3::new(normals[i * 3], normals[i * 3 + 1], normals[i * 3 + 2]);
                if n.norm_squared() > 1e-6 {
                    n = n.normalize();
                    normals[i * 3] = n.x;
                    normals[i * 3 + 1] = n.y;
                    normals[i * 3 + 2] = n.z;
                } else {
                    normals[i * 3] = 0.0;
                    normals[i * 3 + 1] = 0.0;
                    normals[i * 3 + 2] = -1.0;
                }
            }

            self.normals = Some(normals);
        }
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
        writeln!(writer, "format binary_little_endian 1.0")?;
        writeln!(writer, "comment Created by depthmesh")?;

        writeln!(writer, "element vertex {}", num_vertices)?;
        writeln!(writer, "property float x")?;
        writeln!(writer, "property float y")?;
        writeln!(writer, "property float z")?;

        if self.normals.is_some() {
            writeln!(writer, "property float nx")?;
            writeln!(writer, "property float ny")?;
            writeln!(writer, "property float nz")?;
        }

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
            writer.write_all(&self.vertices[i * 3].to_le_bytes())?;
            writer.write_all(&self.vertices[i * 3 + 1].to_le_bytes())?;
            writer.write_all(&self.vertices[i * 3 + 2].to_le_bytes())?;

            if let Some(normals) = &self.normals {
                writer.write_all(&normals[i * 3].to_le_bytes())?;
                writer.write_all(&normals[i * 3 + 1].to_le_bytes())?;
                writer.write_all(&normals[i * 3 + 2].to_le_bytes())?;
            }

            if has_texcoords {
                writer.write_all(&self.texcoords[i * 2].to_le_bytes())?;
                writer.write_all(&(1.0 - self.texcoords[i * 2 + 1]).to_le_bytes())?;
            }

            for col in &extra_cols {
                writer.write_all(&col[i].to_le_bytes())?;
            }
        }

        for face in self.indices.chunks_exact(3) {
            writer.write_all(&[3u8])?;

            writer.write_all(&(face[0] as i32).to_le_bytes())?;
            writer.write_all(&(face[1] as i32).to_le_bytes())?;
            writer.write_all(&(face[2] as i32).to_le_bytes())?;
        }

        writer.flush()?;
        Ok(())
    }
}

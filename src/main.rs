mod cli;
mod mesh;
mod utils;
use anyhow::Result;
use clap::Parser;
use cli::Args;
use mesh::Mesh;
use nalgebra::{Matrix3, Matrix4};

fn main() -> Result<()> {
    let args = Args::parse();

    let depth = image::open(args.depth)?.to_rgb32f();

    let intrinsic = {
        let matrix_data: Vec<f32> = args
            .intrinsic
            .split(',')
            .filter_map(|s| s.trim().parse::<f32>().ok())
            .collect();

        if matrix_data.len() != 9 {
            return Err(anyhow::anyhow!(
                "Intrinsic matrix data must contain 9 values"
            ));
        }

        Matrix3::from_row_slice(&matrix_data)
    };

    let mut mesh = Mesh::new(
        depth,
        args.threshold,
        intrinsic,
        args.scale,
        args.reverse_z,
        args.distance,
    )?;

    let src = match args.source_pose {
        Some(pose) => {
            let matrix_data: Vec<f32> = pose
                .split(',')
                .filter_map(|s| s.trim().parse::<f32>().ok())
                .collect();
            if matrix_data.len() != 16 {
                return Err(anyhow::anyhow!("Pose matrix must contain 16 values"));
            }
            Some(Matrix4::from_row_slice(&matrix_data))
        }
        None => None,
    };

    let target = match args.target_pose {
        Some(pose) => {
            let matrix_data: Vec<f32> = pose
                .split(',')
                .filter_map(|s| s.trim().parse::<f32>().ok())
                .collect();
            if matrix_data.len() != 16 {
                return Err(anyhow::anyhow!("Pose matrix must contain 16 values"));
            }
            Some(Matrix4::from_row_slice(&matrix_data))
        }
        None => None,
    };

    if let (Some(src), Some(target)) = (src, target) {
        mesh.transform(src, target, args.offset)?;
    }

    if args.smooth {
        mesh.smooth(args.iterations, args.lambda);
    }

    if args.optimize {
        mesh.optimize(args.reduction, args.error)?;
    }

    let albedo = args
        .albedo
        .map(|path| image::open(path).map(|img| img.to_rgb32f()))
        .transpose()?;

    if let Some(albedo) = albedo {
        mesh.sample("red", &albedo, 0);
        mesh.sample("green", &albedo, 1);
        mesh.sample("blue", &albedo, 2);
    }

    let normal = args
        .normal
        .map(|path| image::open(path).map(|img| img.to_rgb32f()))
        .transpose()?;

    mesh.compute_normal(normal);

    mesh.write(&args.output.to_string_lossy())?;
    Ok(())
}

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

    let normal = args
        .normal
        .map(|normal| image::open(normal).map(|img| img.to_rgb32f()))
        .transpose()?;

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
        normal,
        args.scale,
        args.reverse_z,
        args.distance,
        args.offset,
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
        mesh.transform(src, target)?;
    }

    if args.smooth {
        mesh.smooth(args.iterations, args.lambda);
    }
    if args.optimize {
        mesh.optimize(args.reduction, args.error)?;
    }

    mesh.write(&args.output.to_string_lossy())?;
    Ok(())
}

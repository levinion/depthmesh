mod cli;
mod mesh;
mod utils;
use anyhow::{Result, ensure};
use clap::Parser;
use cli::Args;
use mesh::Mesh;
use nalgebra::{Matrix3, Matrix4};

fn main() -> Result<()> {
    let args = Args::parse();

    let n = args.depth.len();

    // assert input
    ensure!(
        args.threshold.len() == n,
        "The number of depthes and thresholds must match. (depth: {}, threshold: {})",
        n,
        args.threshold.len()
    );

    ensure!(
        args.intrinsic.len() == n,
        "The number of depthes and intrinsics values must match. (depth: {}, proj: {})",
        n,
        args.intrinsic.len()
    );

    ensure!(
        args.scale.len() == n,
        "The number of depthes and scale values must match. (depth: {}, scale: {})",
        n,
        args.scale.len()
    );

    if !args.normal.is_empty() {
        ensure!(
            args.normal.len() == n,
            "The number of normal maps must match the number of depthes. (depth: {}, normal: {})",
            n,
            args.normal.len()
        );
    }

    if !args.pose.is_empty() {
        ensure!(
            args.pose.len() == n,
            "The number of pose maps must match the number of depthes. (depth: {}, view: {})",
            n,
            args.pose.len()
        );
    }

    // collect args into meshes
    let meshes = {
        let meshes: Result<Vec<Mesh>> = (0..n)
            .map(|i| {
                let img = image::open(&args.depth[i])
                    .map_err(|e| {
                        anyhow::anyhow!(
                            "Failed to open input image '{}': {}",
                            args.depth[i].display(),
                            e
                        )
                    })?
                    .to_luma32f();

                let normal = if !args.normal.is_empty() {
                    Some(
                        image::open(&args.normal[i])
                            .map_err(|e| {
                                anyhow::anyhow!(
                                    "Failed to open normal map '{}': {}",
                                    args.normal[i].display(),
                                    e
                                )
                            })?
                            .to_rgb32f(),
                    )
                } else {
                    None
                };

                let intrinsic = {
                    let matrix_data: Vec<f32> = args.intrinsic[i]
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

                let pose = if !args.pose.is_empty() {
                    let matrix_data: Vec<f32> = args.pose[i]
                        .split(',')
                        .filter_map(|s| s.trim().parse::<f32>().ok())
                        .collect();

                    if matrix_data.len() != 16 {
                        return Err(anyhow::anyhow!("Pose matrix must contain 16 values"));
                    }

                    Some(Matrix4::from_row_slice(&matrix_data))
                } else {
                    None
                };

                Mesh::new(
                    img,
                    args.threshold[i],
                    intrinsic,
                    normal,
                    pose,
                    args.scale[i],
                    args.reverse_z,
                    args.distance,
                )
                .map_err(|e| anyhow::anyhow!("Failed to create mesh from index {}: {}", i, e))
            })
            .collect();

        meshes?
    };

    let mut mesh = {
        let mut meshes = meshes.into_iter();

        let mut mesh = meshes
            .next()
            .ok_or_else(|| anyhow::anyhow!("Empty meshes"))?;

        for m in meshes {
            mesh.merge(m)?;
        }

        mesh
    };

    if args.normalize {
        mesh.normalize();
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

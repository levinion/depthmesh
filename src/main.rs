mod cli;
mod mesh;
mod utils;
use anyhow::Result;
use clap::Parser;
use cli::Args;
use mesh::Mesh;

fn main() -> Result<()> {
    let args = Args::parse();

    let img = image::open(args.input)?.to_luma32f();

    let normal = args
        .normal
        .map(|path| image::open(path).map(|img| img.to_rgb32f()))
        .transpose()?;

    let mask = args
        .mask
        .map(|path| image::open(path).map(|img| img.to_luma32f()))
        .transpose()?;

    let mut mesh = Mesh::new(
        img,
        args.scale,
        args.threshold,
        args.normalize,
        args.fov,
        args.fx,
        args.fy,
        args.cx,
        args.cy,
        normal,
        mask,
    )?;

    if args.smooth {
        mesh.smooth(args.iterations, args.lambda);
    }
    if args.optimize {
        mesh.optimize(args.reduction, args.error)?;
    }

    mesh.write(&args.output.to_string_lossy())?;
    Ok(())
}

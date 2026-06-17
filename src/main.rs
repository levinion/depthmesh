mod cli;
mod mesh;
mod utils;
use clap::Parser;
use cli::Args;
use mesh::Mesh;

fn main() {
    let args = Args::parse();
    let img = image::open(args.input).unwrap().to_luma8();
    let normal = args.normal.map(|path| image::open(path).unwrap().to_rgb8());
    let mut mesh = Mesh::new(img, args.scale, normal);
    if args.smooth {
        mesh.smooth(args.iterations, args.lambda);
    }
    if args.optimize {
        mesh.optimize(args.reduction, args.error);
    }
    mesh.write(&args.output.to_string_lossy());
}

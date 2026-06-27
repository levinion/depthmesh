use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    pub input: PathBuf,

    #[arg(short, long, default_value = "mesh.obj")]
    pub output: PathBuf,

    #[arg(short, long)]
    pub normal: Option<PathBuf>,

    #[arg(short, long)]
    pub mask: Option<PathBuf>,

    #[arg(short, long, default_value_t = 0.05)]
    pub threshold: f32,

    #[arg(short, long, default_value_t = 1.0, allow_hyphen_values(true))]
    pub scale: f32,

    #[arg(short, long)]
    pub fov: Option<f32>,

    #[arg(long)]
    pub fx: Option<f32>,

    #[arg(long)]
    pub fy: Option<f32>,

    #[arg(long)]
    pub cx: Option<f32>,

    #[arg(long)]
    pub cy: Option<f32>,

    #[arg(long, default_value_t = false)]
    pub optimize: bool,

    #[arg(long, default_value_t = 0.1)]
    pub reduction: f32,

    #[arg(long, default_value_t = 0.01)]
    pub error: f32,

    #[arg(long, default_value_t = false)]
    pub smooth: bool,

    #[arg(long, default_value_t = false)]
    pub normalize: bool,

    #[arg(long, default_value_t = 0.1)]
    pub lambda: f32,

    #[arg(long, default_value_t = 10)]
    pub iterations: usize,
}

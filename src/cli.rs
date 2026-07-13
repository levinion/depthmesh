use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, action = clap::ArgAction::Append)]
    pub depth: Vec<PathBuf>,

    #[arg(short, long, default_value = "mesh.obj")]
    pub output: PathBuf,

    #[arg(short, long, action = clap::ArgAction::Append)]
    pub normal: Vec<PathBuf>,

    #[arg(short, long, action = clap::ArgAction::Append)]
    pub threshold: Vec<f32>,

    #[arg(short, long, action = clap::ArgAction::Append)]
    pub intrinsic: Vec<String>,

    #[arg(short, long, action = clap::ArgAction::Append)]
    pub pose: Vec<String>,

    #[arg(short, long, action = clap::ArgAction::Append)]
    pub scale: Vec<f32>,

    #[arg(short, long, default_value_t = false)]
    pub reverse_z: bool,

    #[arg(short = 'D', long, default_value_t = false)]
    pub distance: bool,

    #[arg(long, default_value_t = false)]
    pub optimize: bool,

    #[arg(long, default_value_t = 0.1)]
    pub reduction: f32,

    #[arg(long, default_value_t = 0.01)]
    pub error: f32,

    #[arg(long, default_value_t = false)]
    pub smooth: bool,

    #[arg(long, default_value_t = 0.1)]
    pub lambda: f32,

    #[arg(long, default_value_t = 10)]
    pub iterations: usize,
}

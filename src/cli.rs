use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    pub depth: PathBuf,

    #[arg(short, long)]
    pub output: PathBuf,

    #[arg(short, long)]
    pub normal: Option<PathBuf>,

    #[arg(short, long, default_value_t = 0.1)]
    pub threshold: f32,

    #[arg(short, long, allow_hyphen_values = true)]
    pub intrinsic: String,

    #[arg(long, allow_hyphen_values = true)]
    pub source_pose: Option<String>,

    #[arg(long, allow_hyphen_values = true)]
    pub target_pose: Option<String>,

    #[arg(long, default_value_t = 0.0, allow_negative_numbers = true)]
    pub offset: f32,

    #[arg(short, long, default_value_t = 1.0, allow_negative_numbers = true)]
    pub scale: f32,

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

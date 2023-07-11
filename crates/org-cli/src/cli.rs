use clap::{Parser, ValueEnum, ValueHint};

#[derive(Parser)]
#[command(name = "org-rust")]
#[command(author = "Laith Bahodi <laithbahodi@gmail.com>")]
#[command(about = "Exporter for Org Mode Content")]
#[command(author, version, about, long_about=None)]
pub struct Cli {
    #[arg(value_enum)]
    pub backend: Backend,

    /// input file path
    #[arg(value_hint = ValueHint::FilePath)]
    pub file: Option<String>,

    /// output file path
    #[arg(short, long, value_hint = ValueHint::FilePath)]
    pub output: Option<String>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum Backend {
    Html,
    Org,
}

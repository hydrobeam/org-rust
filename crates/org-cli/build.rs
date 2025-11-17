use clap::CommandFactory;
use clap_complete::{Shell, generate_to};
use std::{env, io};
include!("src/cli.rs");

fn main() -> Result<(), io::Error> {
    let outdir = match env::var_os("OUT_DIR") {
        None => return Ok(()),
        Some(outdir) => outdir,
    };

    // let built = Cli::command();
    let mut built = Cli::command();

    for &shell in Shell::value_variants() {
        generate_to(shell, &mut built, "org-rust", &outdir)?;
    }

    let man = clap_mangen::Man::new(built);
    let mut buffer: Vec<u8> = Vec::new();
    man.render(&mut buffer)?;
    std::fs::write(std::path::PathBuf::from(outdir).join("org-rust.1"), buffer)?;

    Ok(())
}

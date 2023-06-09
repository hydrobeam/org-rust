use clap::CommandFactory;
use clap_complete::{
    generate_to,
    Shell,
};
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

    Ok(())
}

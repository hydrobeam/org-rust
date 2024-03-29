use cli::Backend;
use lazy_format::prelude::*;
use std::fs::{read_to_string, OpenOptions};
use std::io::{stdin, Read, Write};

use clap::Parser;
use org_exporter::Exporter;
mod cli;

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = cli::Cli::parse();
    let mut input_source: String = String::new();
    let mut out = String::new();

    match cli.file {
        None => {
            stdin().lock().read_to_string(&mut input_source)?;
        }
        Some(ref file) => input_source = read_to_string(file)?,
    }

    match cli.backend {
        Backend::Html => {
            org_exporter::Html::export_buf(&input_source, &mut out)?;
        }
        Backend::Org => {
            org_exporter::Org::export_buf(&input_source, &mut out)?;
        }
    };

    // add default html structure when writing html to a file
    if let Backend::Html = cli.backend {
        if let Some(loc) = cli.output {
            // lazy format so the entire output isn't reallocated just before writing to a file
            let ret_str = lazy_format!(
                r#"
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta http-equiv="X-UA-Compatible" content="ie=edge">
  </head>
</head>
<body>
{out}
<body>
</html>
"#
            );
            let mut fs = OpenOptions::new().write(true).create(true).open(&loc)?;
            fs.write_fmt(format_args!("{ret_str}"))?;
        } else {
            print!("{out}");
        }
    } else {
        print!("{out}");
    }

    Ok(())
}

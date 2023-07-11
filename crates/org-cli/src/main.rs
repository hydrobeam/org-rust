use cli::Backend;
use lazy_format::prelude::*;
use std::fs::{read_to_string, OpenOptions};
use std::io::{stdin, Read, Result, Write};

use clap::Parser;
use org_exporter::Exporter;
mod cli;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    let mut input_source: String = String::new();
    let mut out = String::new();

    match cli.file {
        None => {
            stdin().lock().read_to_string(&mut input_source)?;
        }
        Some(file) => {
            input_source = read_to_string(file)?;
        }
    }

    match cli.backend {
        Backend::Html => {
            org_exporter::Html::export_buf(&input_source, &mut out).unwrap();
        }
        Backend::Org => {
            org_exporter::Org::export_buf(&input_source, &mut out).unwrap();
        }
    };

    // add default html structure when writing html to a file
    if let Backend::Html = cli.backend {
        if let Some(loc) = cli.output {
            // lazy format so the entire output isn't reallocated just before writing to a file
            let ret_str = lazy_format!(
                r#"<!doctype html>
<html>
    <meta charset="utf-8"/>
<head>
</head>
<body>
{out}
<body>
</html>
"#
            );
            let mut fs = OpenOptions::new().write(true).open(&loc)?;
            fs.write_fmt(format_args!("{ret_str}"))?;
        } else {
            print!("{out}");
        }
    } else {
        print!("{out}");
    }

    Ok(())
}

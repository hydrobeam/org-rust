use lazy_format::prelude::*;
use std::collections::HashMap;
use std::fs::{self, read_to_string, File, OpenOptions};
use std::io::{stdin, stdout, BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;

use clap::Parser;
use org_exporter::Exporter;

use crate::cli::Backend;
mod cli;
mod eat;
mod template;

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

/// Macro to prefer parameters passed in via the cli over a config file / defaults
macro_rules! config_dupl {
    ($new_var:ident, $field:ident, $cli_params:ident, $config_params:ident) => {
        let $new_var = match $cli_params.$field {
            None => $config_params.$field,
            r => r,
        };
    };
}

enum InpType<'a> {
    File(Box<dyn BufRead>),
    Dir(&'a Path),
}

enum OutType<'a> {
    File(Box<dyn Write>),
    Dir(&'a Path),
}

// we only want to hold one file in memory
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli_params = cli::Cli::parse();
    let config_params: cli::Cli;

    if let Some(config_path) = cli_params.config {
        let a = read_to_string(config_path)?;
        config_params = toml::from_str(&a)?;
    } else {
        config_params = cli::Cli::default();
    }

    config_dupl!(backend, backend, cli_params, config_params);
    config_dupl!(input_path, input, cli_params, config_params);
    config_dupl!(output_path, output, cli_params, config_params);
    // config_dupl!(template_path, templates, cli_params, config_params);

    let backend = match backend {
        Some(b) => b,
        None => Backend::default(),
    };

    // fn handle_file that eats a bufreader, hmm
    //
    // but we can't do that because we need to establish it's a file.
    // maybe an enum that eats a bufreader and a path, if it's a path it's a dir
    // bufreader => file
    let src = match input_path {
        None => InpType::File(Box::new(BufReader::new(stdin().lock()))),
        Some(ref file) => {
            // let mut fs = OpenOptions::new().write(true).create(true).open(file)?;
            let f = std::path::Path::new(file);

            // let meta = std::fs::metadata(file)?;
            if f.is_dir() {
                InpType::Dir(f)
            } else {
                InpType::File(Box::new(BufReader::new(std::fs::File::open(f)?)))
            }
        }
    };

    // if input is a dir, then output is a dir
    // if output is none, then output is stdout regardless

    let mut dest = match output_path {
        None => OutType::File(Box::new(BufWriter::new(stdout()))),
        Some(ref file) => {
            let f = std::path::Path::new(file);
            match src {
                // it's fine to open the file now (i.e. during the duration of parsing)
                // since we're only working with one file. meaning we're processing at most
                // 1 file, which should be relatively quick.
                //
                // HACK: would like to open the file only when we've finished parsing.
                InpType::File(_) => {
                    OutType::File(Box::new(BufWriter::new(std::fs::File::open(f)?)))
                }
                InpType::Dir(_) => OutType::Dir(f),
            }
        }
    };

    let mut file_contents = String::new();
    let mut exported_content = String::new();
    match src {
        InpType::File(mut f) => {
            let _num_bytes = f.read_to_string(&mut file_contents)?;

            let parser_output = org_parser::parse_org(&file_contents);

            backend.export(&parser_output, &mut exported_content)?;

            if let Some(template_path) = parser_output.keywords.get("template_path") {
                // do shit
            }

            // only files are allowed if inp is a file
            if let OutType::File(mut f) = dest {
                f.write(&exported_content.as_bytes())?;
            } else {
                unreachable!()
            }

            // write to file
        }
        InpType::Dir(src_dir) => {
            // farm up files
            let mut paths = Vec::new();
            let mut dirs = vec![src_dir.to_path_buf()];

            while let Some(dir) = dirs.pop() {
                for entry in fs::read_dir(dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() {
                        paths.push(path)
                    } else if path.is_dir() {
                        dirs.push(path)
                    } else {
                        todo!("unimplemented file type")
                        // path.is_
                    }
                }
            }

            for file_path in &paths {
                if file_path.ends_with(".org") {
                    let mut f = std::fs::File::open(file_path)?;
                    let _num_bytes = f.read_to_string(&mut file_contents)?;
                    let parser_output = org_parser::parse_org(&file_contents);
                    backend.export(&parser_output, &mut exported_content)?;
                    if let Some(template_path) = parser_output.keywords.get("template_path") {
                        // do shit
                    }
                    match dest {
                        // this is stdout
                        OutType::File(ref mut f) => {
                            f.write(&exported_content.as_bytes())?;
                        }
                        OutType::Dir(dest_path) => {
                            // origin: ./dest/d/e.org
                            // goal: strip the "./dest" prefix, and append /d/e.org to the destination. use fs::create_dir_all.
                            //
                            // output: ./out/d/e.html
                            let stripped_path = file_path.strip_prefix(src_dir)?;

                            let mut full_output_path = dest_path.join(stripped_path);
                            full_output_path.set_extension(backend.extension());
                        }
                    }
                } else {
                    // if not org, do nothing and just copy it
                    let stripped_path = file_path.strip_prefix(src_dir)?;
                    if let OutType::Dir(dest_path) = dest {
                        let full_output_path = dest_path.join(stripped_path);
                        fs::copy(file_path, full_output_path)?;
                    }
                }

                file_contents.clear();
            }
        }
    }

    // match backend {
    //     Backend::Html => org_parser::parse_org(input)
    //     Backend::Org => todo!(),
    // }
    // let templates : HashMap<String, Template> = process_templates(template_path);

    // let export_func =
    // match cli.backend {
    //     None | Some(Backend::Html) => {
    //         org_exporter::Html::export_buf(&input_source, &mut out)?;
    //     }
    //     Some(Backend::Org) => {
    //         org_exporter::Org::export_buf(&input_source, &mut out)?;
    //     }
    // };
    // match cli.backend {
    //     None | Some(Backend::Html) => {
    //         org_exporter::Html::export_buf(&input_source, &mut out)?;
    //     }
    //     Some(Backend::Org) => {
    //         org_exporter::Org::export_buf(&input_source, &mut out)?;
    //     }
    // };

    //    // add default html structure when writing html to a file
    //     if let Backend::Html = cli.backend {
    //         if let Some(loc) = cli.output {
    //             // lazy format so the entire output isn't reallocated just before writing to a file
    //             let ret_str = lazy_format!(
    //                 r#"
    // <!DOCTYPE html>
    // <html lang="en">
    //   <head>
    //     <meta charset="UTF-8">
    //     <meta name="viewport" content="width=device-width, initial-scale=1.0">
    //   </head>
    // </head>
    // <body>
    // {out}
    // <body>
    // </html>
    // "#
    //             );
    //             let mut fs = OpenOptions::new().write(true).create(true).open(&loc)?;
    //             fs.write_fmt(format_args!("{ret_str}"))?;
    //         } else {
    //             print!("{out}");
    //         }
    //     } else {
    //         print!("{out}");
    //     }

    Ok(())
}

use anyhow::bail;
use org_exporter::ConfigOptions;
use std::borrow::Cow;
use std::fs::{self, read_to_string, OpenOptions};
use std::io::{stdout, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use template::Template;
use types::{CliError, InpType, OutType};
use utils::mkdir_recursively;

use clap::Parser;

mod template;
use crate::cli::Backend;
mod cli;
mod types;
mod utils;

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

/// Function that works through the entire pipeline
fn run() -> anyhow::Result<()> {
    let cli_params = cli::Cli::parse();
    let config_params: cli::Cli;

    if let Some(config_path) = cli_params.config {
        let path = Path::new(&config_path);
        let conf = read_to_string(path).map_err(|e| {
            CliError::from(e)
                .with_path(path)
                .with_cause(&format!("failed to read config file: {}", path.display()))
        })?;

        config_params = toml::from_str(&conf)?;
    } else {
        config_params = cli::Cli::default();
    }

    // prefer cli params to config params
    let backend = match cli_params.backend {
        None => config_params.backend,
        r => r,
    };
    let output_path = if cli_params.output == "" {
        config_params.output
    } else {
        cli_params.output
    };
    let input_path = if cli_params.input == "" {
        config_params.input
    } else {
        cli_params.input
    };

    let verbose = if cli_params.verbose {
        cli_params.verbose
    } else {
        config_params.verbose
    };

    let backend = match backend {
        Some(b) => b,
        None => Backend::default(),
    };

    let f = Path::new(&input_path);
    if !f.exists() {
        bail!("Input path not found: {}", f.display());
    }
    let src = if f.is_dir() {
        InpType::Dir(f)
    } else {
        InpType::File(f)
    };

    // if input is a dir, then output is a dir
    // if output is none, then output is stdout regardless

    let f = Path::new(&output_path);
    let dest = match src {
        InpType::File(_) => OutType::File(f),
        InpType::Dir(_) => {
            if !f.exists() {
                mkdir_recursively(f)?;
            }
            OutType::Dir(f)
        }
    };

    let mut file_contents = String::new();
    let mut exported_content = String::new();

    // vecs that hold dirs/files that need to be processed
    let mut paths = Vec::new();
    let mut dirs = Vec::new();

    match src {
        InpType::File(p) => paths.push(p.to_path_buf()),
        InpType::Dir(p) => dirs.push(p.to_path_buf()),
    }

    // recursively process directories to find all containing files
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir).map_err(|e| CliError::from(e).with_path(&dir))? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                paths.push(path)
            } else if path.is_dir() && path.is_symlink() {
                // we don't want to duplicate the contents of symlinks to dirs,
                // just copy the dir as is.
                // REVIEW: what about symlinks to dirs of org files?
                paths.push(path)
            } else if path.is_dir() {
                dirs.push(path)
            } else {
                bail!("unsupported file type: {}", path.display())
            }
        }
    }

    // PERF: avoid overloading syscalls if lots of files are processed
    let mut stdout = BufWriter::new(stdout());

    // main loop to export files
    for file_path in &paths {
        if verbose {
            writeln!(stdout, "input: {}", file_path.display()).map_err(|e| CliError::from(e))?;
        }
        if file_path.extension().is_some_and(|x| x == "org") {
            let mut input_file = std::fs::File::open(file_path)
                .map_err(|e| CliError::from(e).with_path(&file_path))?;
            let _num_bytes = input_file.read_to_string(&mut file_contents).map_err(|e| {
                CliError::from(e)
                    .with_path(file_path)
                    .with_cause("failed to read input file")
            });

            let mut parser_output = org_parser::parse_org(&file_contents);
            // convert .org links to .extension links
            for item in parser_output.pool.iter_mut() {
                if let org_parser::Expr::RegularLink(expr) = &mut item.obj {
                    match &mut expr.path.obj {
                        org_parser::object::PathReg::PlainLink(l) => {
                            let p = &mut l.path;
                            if let Some(v) = p.strip_suffix(".org") {
                                let mut v = v.to_owned();
                                v.push_str(backend.extension());
                                *p = v.into();
                            }
                        }
                        org_parser::object::PathReg::File(l)
                        | org_parser::object::PathReg::Unspecified(l) => {
                            if let Some(v) = l.strip_suffix(".org") {
                                use std::fmt::Write;
                                let mut v = v.to_owned();
                                write!(v, ".{}", backend.extension())?;
                                *l = v.into();
                            }
                        }
                        _ => {}
                    }
                }
            }

            let conf = ConfigOptions::new(Some(file_path.to_path_buf()));
            backend.export(&parser_output, &mut exported_content, conf)?;

            if let Some(template_path) = parser_output.keywords.get("template_path") {
                // evaluate relative paths if needed
                let template_path = Path::new(template_path);
                let template_path: Cow<Path> = if template_path.is_relative() {
                    file_path
                        .parent()
                        .unwrap()
                        .join(template_path)
                        .canonicalize()
                        .map_err(|e| {
                            CliError::from(e)
                                .with_path(&file_path.parent().unwrap().join(template_path))
                                .with_cause(&format!(
                                    "Failed to locate template_path from: {}",
                                    file_path.display()
                                ))
                        })?
                        .into()
                } else {
                    template_path.into()
                };

                let template_contents = std::fs::read_to_string(&template_path).map_err(|e| {
                    CliError::from(e)
                        .with_path(&template_path)
                        .with_cause("error with opening template file")
                })?;
                // exported_content =
                let mut t = Template::form_template(
                    &parser_output,
                    &template_path,
                    &template_contents,
                    &exported_content,
                )?;
                exported_content = t.process()?;
            }

            // the destination we are writing to
            let mut full_output_path: PathBuf;
            match dest {
                OutType::File(ref output_file) => {
                    full_output_path = output_file.to_path_buf();
                }
                OutType::Dir(dest_path) => {
                    // origin: ./path/to/d/e.org
                    // goal: strip the "./path/to/" prefix, and append /d/e.org to the destination.
                    //
                    // output: ./out/d/e.html
                    let stripped_path = if let InpType::Dir(src_dir) = src {
                        file_path.strip_prefix(src_dir)?
                    } else {
                        unreachable!()
                    };
                    full_output_path = dest_path.join(stripped_path);
                    full_output_path.set_extension(backend.extension());
                }
            }

            mkdir_recursively(&full_output_path.parent().unwrap())?;
            // truncate is needed to fully overwrite file contents
            let mut opened = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&full_output_path)
                .map_err(|e| {
                    CliError::from(e)
                        .with_path(&full_output_path)
                        .with_cause("error in writing to destination file")
                })?;
            opened.write(&exported_content.as_bytes())?;
            if verbose {
                writeln!(
                    stdout,
                    " -- processed: {}\n",
                    full_output_path.canonicalize()?.display()
                )?
            }
        } else {
            // if not org, do nothing and just copy it
            let stripped_path = if let InpType::Dir(src_dir) = src {
                file_path.strip_prefix(src_dir)?
            } else {
                unreachable!()
            };
            if let OutType::Dir(dest_path) = dest {
                let full_output_path = dest_path.join(stripped_path);
                mkdir_recursively(full_output_path.parent().unwrap())?;

                // fs::copy doesn't handle symlinked dirs. we do it ourselves
                if file_path.is_symlink() && file_path.is_dir() {
                    let t = fs::read_link(file_path)?;
                    if full_output_path.exists() {
                        fs::remove_dir_all(&full_output_path)?;
                    }
                    std::os::unix::fs::symlink(t, &full_output_path)?;
                    if verbose {
                        writeln!(
                            stdout,
                            " -- symlinked: {}\n",
                            &full_output_path.canonicalize()?.display()
                        )
                        .map_err(|e| CliError::from(e))?;
                    }
                } else {
                    fs::copy(file_path, &full_output_path).map_err(|e| {
                        CliError::from(e)
                            .with_path(&file_path)
                            .with_cause("error in copying file to destination")
                    })?;
                    if verbose {
                        writeln!(
                            stdout,
                            " -- copied: {}\n",
                            &full_output_path.canonicalize()?.display()
                        )
                        .map_err(|e| CliError::from(e))?;
                    }
                }
            }
        }
        file_contents.clear();
        exported_content.clear();
    }

    Ok(())
}

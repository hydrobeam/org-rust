use anyhow::bail;
use std::env::current_dir;
use std::fs::{self, read_to_string, OpenOptions};
use std::io::{stdout, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use utils::normalize_path;

use clap::Parser;

use thiserror::Error;

use crate::cli::Backend;
mod cli;
mod eat;
mod template;
mod utils;

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

#[derive(Debug)]
enum InpType<'a> {
    File(&'a Path),
    Dir(&'a Path),
}

#[derive(Debug)]
enum OutType<'a> {
    File(&'a Path),
    Dir(&'a Path),
}

#[derive(Error, Debug)]
enum CliError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{path}: {err}")]
    WithPath { err: Box<CliError>, path: PathBuf },
    #[error("{cause}: {err}")]
    WithCause { err: Box<CliError>, cause: String },
}

impl CliError {
    fn with_path(self, path: &Path) -> Self {
        Self::WithPath {
            err: Box::new(self),
            path: normalize_path(path),
        }
    }
    fn with_cause(self, cause: &str) -> Self {
        Self::WithCause {
            err: Box::new(self),
            cause: cause.to_string(),
        }
    }
}

// we only want to hold one file in memory
fn run() -> anyhow::Result<()> {
    let cli_params = cli::Cli::parse();
    let config_params: cli::Cli;

    if let Some(config_path) = cli_params.config {
        let a = read_to_string(config_path)?;

        config_params = toml::from_str(&a)?;
    } else {
        config_params = cli::Cli::default();
    }

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

    let backend = match backend {
        Some(b) => b,
        None => Backend::default(),
    };

    // fn handle_file that eats a bufreader, hmm
    //
    // but we can't do that because we need to establish it's a file.
    // maybe an enum that eats a bufreader and a path, if it's a path it's a dir
    // bufreader => file
    let f = std::path::Path::new(&input_path);
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

    let f = std::path::Path::new(&output_path);
    let dest = match src {
        InpType::File(_) => OutType::File(f),
        InpType::Dir(_) => {
            if !f.exists() {
                std::fs::create_dir_all(f).map_err(|e| CliError::from(e).with_path(f))?;
            }
            OutType::Dir(f)
        }
    };

    let mut file_contents = String::new();
    let mut exported_content = String::new();

    // farm up files
    let mut paths = Vec::new();
    let mut dirs = Vec::new();

    match src {
        InpType::File(p) => paths.push(p.to_path_buf()),
        InpType::Dir(p) => dirs.push(p.to_path_buf()),
    }

    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir).map_err(|e| CliError::from(e).with_path(&dir))? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
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

    for file_path in &paths {
        write!(stdout, "input: {}", file_path.display()).map_err(|e| CliError::from(e))?;
        if let Some(ext) = file_path.extension() {
            if ext == "org" {
                let mut input_file = std::fs::File::open(file_path)
                    .map_err(|e| CliError::from(e).with_path(&file_path))?;
                let _num_bytes = input_file.read_to_string(&mut file_contents).map_err(|e| {
                    CliError::from(e)
                        .with_path(file_path)
                        .with_cause("failed to read input file")
                });
                let parser_output = org_parser::parse_org(&file_contents);
                backend.export(&parser_output, &mut exported_content)?;
                if let Some(template_path) = parser_output.keywords.get("template_path") {
                    let prev = current_dir()
                        .map_err(|e| CliError::from(e).with_cause("failed to getcwd"))?;
                    // file_path ought to exist and can't be the root since it's been read from.
                    // no error possible
                    let target_dir = file_path.parent().unwrap();
                    std::env::set_current_dir(target_dir).map_err(|e| {
                        CliError::from(e)
                            .with_path(target_dir)
                            .with_cause("failed to chdir")
                    })?;

                    exported_content = process_template(template_path, &exported_content)?;
                    std::env::set_current_dir(prev)
                        .map_err(|e| CliError::from(e).with_cause("failed to chdir"))?;
                }

                // the destination we are writing to
                let mut full_output_path: PathBuf;
                match dest {
                    OutType::File(ref output_file) => {
                        full_output_path = output_file.to_path_buf();
                    }
                    OutType::Dir(dest_path) => {
                        // origin: ./dest/d/e.org
                        // goal: strip the "./dest" prefix, and append /d/e.org to the destination. use fs::create_dir_all.
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

                let mut opened = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .open(&full_output_path)
                    .map_err(|e| {
                        CliError::from(e)
                            .with_path(&full_output_path)
                            .with_cause("error in writing to destination file")
                    })?;
                opened.write(&exported_content.as_bytes())?;
                writeln!(stdout, " -- destination: {}", file_path.display())
                    .map_err(|e| CliError::from(e))?;
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
                fs::copy(file_path, full_output_path).map_err(|e| {
                    CliError::from(e)
                        .with_path(&file_path)
                        .with_cause("error in copying file to destination")
                });
            }
        }

        file_contents.clear();
    }

    Ok(())
}

fn process_template(template_path: &str, exported_output: &str) -> Result<String, CliError> {
    let f = std::path::Path::new(template_path);
    let template_contents = std::fs::read_to_string(f).map_err(|e| {
        CliError::from(e)
            .with_path(f)
            .with_cause("error with opening template file")
    })?;

    // the regex is checked at compile time and won't exceed the size limits + is valid
    let re = regex::Regex::new(r#"\{\{\{content\}\}\}"#).unwrap();
    let a = re.replace(&template_contents, exported_output);

    Ok(a.to_string())
}

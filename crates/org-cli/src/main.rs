use lazy_format::prelude::*;
use serde::de::IntoDeserializer;
use std::collections::HashMap;
use std::env::current_dir;
use std::error::Error;
use std::fs::{self, read_to_string, File, OpenOptions};
use std::io::{stdin, stdout, BufRead, BufReader, BufWriter, Read, Write};
use std::path::{self, Path, PathBuf};
use utils::normalize_path;

use clap::Parser;
use org_exporter::Exporter;

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
            let f = std::path::Path::new(file);
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
                    let opened = OpenOptions::new().create(true).write(true).open(f)?;
                    OutType::File(Box::new(BufWriter::new(opened)))
                }
                InpType::Dir(_) => {
                    if !f.exists() {
                        std::fs::create_dir_all(f)?;
                    }
                    OutType::Dir(f)
                }
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
                let prev =
                    current_dir().map_err(|e| CliError::from(e).with_cause("failed to getcwd"))?;
                // file_path ought to exist and can't be the root since it's been read from.
                // no error possible
                let target_dir = file_path.parent().unwrap();
                std::env::set_current_dir(target_dir).map_err(|e| {
                    CliError::from(e)
                        .with_path(target_dir)
                        .with_cause("failed to chdir")
                })?;

                exported_content = process_template(template_path, &exported_content)?;
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
                for entry in fs::read_dir(&dir).map_err(|e| CliError::from(e).with_path(&dir))? {
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
                if let Some(ext) = file_path.extension() {
                    if ext == "org" {
                        let mut input_file = std::fs::File::open(file_path)
                            .map_err(|e| CliError::from(e).with_path(&file_path))?;
                        let _num_bytes =
                            input_file.read_to_string(&mut file_contents).map_err(|e| {
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
                        }
                        match dest {
                            // this is stdout
                            OutType::File(ref mut output_file) => {
                                output_file.write(&exported_content.as_bytes())?;
                            }
                            OutType::Dir(dest_path) => {
                                // origin: ./dest/d/e.org
                                // goal: strip the "./dest" prefix, and append /d/e.org to the destination. use fs::create_dir_all.
                                //
                                // output: ./out/d/e.html
                                let stripped_path = file_path.strip_prefix(src_dir)?;

                                let mut full_output_path = dest_path.join(stripped_path);
                                full_output_path.set_extension(backend.extension());

                                let mut opened = OpenOptions::new()
                                    .create(true)
                                    .write(true)
                                    .open(&full_output_path)
                                    .map_err(|e| {
                                        CliError::from(e)
                                            .with_path(&full_output_path)
                                            .with_cause("error in writing to file")
                                    })?;

                                opened.write(&exported_content.as_bytes()).unwrap();
                            }
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

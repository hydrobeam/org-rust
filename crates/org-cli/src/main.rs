use anyhow::bail;
use std::env::current_dir;
use std::fs::{self, read_to_string, OpenOptions};
use std::io::{stdout, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use types::{CliError, InpType, OutType};
use utils::mkdir_recursively;

use clap::Parser;

use crate::cli::Backend;
use crate::utils::switch_dir;
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
        writeln!(stdout, "input: {}", file_path.display()).map_err(|e| CliError::from(e))?;
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

                    // HACK: switch dirs to allow template file finding using relative paths
                    let target_dir = file_path.parent().unwrap();

                    switch_dir(&target_dir)?;
                    exported_content =
                        process_template(template_path, &exported_content, &parser_output)?;

                    switch_dir(&prev)?;
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
                writeln!(
                    stdout,
                    " -- processed: {}\n",
                    full_output_path.canonicalize()?.display()
                )?
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
                    fs::copy(file_path, &full_output_path).map_err(|e| {
                        CliError::from(e)
                            .with_path(&file_path)
                            .with_cause("error in copying file to destination")
                    })?;
                    writeln!(
                        stdout,
                        " -- copied: {}\n",
                        &full_output_path.canonicalize()?.display()
                    )
                    .map_err(|e| CliError::from(e))?;
                }
            }
        }
        file_contents.clear();
        exported_content.clear();
    }

    Ok(())
}

fn process_template(
    template_path: &str,
    exported_content: &str,
    parser: &org_parser::Parser,
) -> Result<String, CliError> {
    let f = Path::new(template_path);
    let mut template_contents = std::fs::read_to_string(f).map_err(|e| {
        CliError::from(e)
            .with_path(f)
            .with_cause("error with opening template file")
    })?;

    // the regex is checked at compile time and won't exceed the size limits + is valid
    let re = regex::Regex::new(r#"\{\{\{(.*)\}\}\}"#).unwrap();

    let mut matches = Vec::new();
    // collect all matches to {{{.*}}} regex - things we want to replace with keywords
    for captured in re.captures_iter(&template_contents) {
        if let Some(res) = captured.get(1) {
            // we expand the range of the capture to include the {{{}}}
            let start = res.start() - 3;
            let end = res.end() + 3;
            let extract = res.as_str();
            let kw: &str;

            if extract == "content" {
                kw = exported_content;
            } else {
                // &* needed because: https://stackoverflow.com/a/65550108
                kw = if let Some(val) = parser.keywords.get(&*extract) {
                    val
                } else {
                    eprintln!(r#"warning: "{}" not found in keywords"#, extract);
                    ""
                };
            }
            matches.push((start, end, kw));
        }
    }
    // process: we take all our matches and replace them with their respective hits as needed.
    // however, the indices change as we replace a section of the original string, so we keep
    // track of an offset which determines how much the start/end indicies must be adjusted
    //
    // demo:
    // {{{a}}} -> hi.         the new string is smaller, so offset is decreased.
    // {{{a}}} -> long-string the new string is larger, so offset is increased
    //
    // REVIEW: this process is probably slow, maybe a faster solution?

    // offset calculators
    let mut offset = 0;
    let mut diff;
    let mut old_len;
    let mut new_len;

    for (start, end, kw) in matches {
        // "content" is a special case keyword
        let start = (start as isize + offset) as usize;
        let end = (end as isize + offset) as usize;

        template_contents.replace_range(start..end, kw);

        // old_len is length of extract + 6 from {{{}}}
        old_len = (end - start) as isize;
        new_len = kw.len() as isize;
        diff = new_len - old_len;
        offset += diff;
    }

    Ok(template_contents)
}

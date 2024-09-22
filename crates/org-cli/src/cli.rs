use clap::{ArgAction, Parser, ValueEnum, ValueHint};
use org_exporter::{ConfigOptions, Exporter};
use serde::Deserialize;

#[derive(Parser, Deserialize, Default)]
#[command(name = "org-rust")]
#[command(author = "Laith Bahodi <laithbahodi@gmail.com>")]
#[command(about = "Exporter for Org Mode Content")]
#[command(author, version, about, long_about=None)]
pub struct Cli {
    /// Default is html
    #[arg(short, long, value_enum)]
    pub backend: Option<Backend>,

    /// Input path
    ///
    /// If the input is a directory, `org-rust` will walk and export every file
    /// to the output directory maintaining the directory structure.
    #[arg(value_hint = ValueHint::FilePath)]
    pub input: String,

    /// Output path
    ///
    /// The output type corresponds to the type of the input. I.e. if the input path is a file
    /// then the output path will be a file, same for a directory.
    #[arg(short, long, value_hint = ValueHint::AnyPath)]
    pub output: String,

    /// Path to config file
    ///
    /// CLI params are preferred over config file params
    #[arg(short, long, value_hint = ValueHint::FilePath)]
    pub config: Option<String>,

    /// Display details of the program's execution
    ///
    /// Prints files that are being processed / copied
    #[arg(short, long, action = ArgAction::SetTrue)]
    pub verbose: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Deserialize)]
pub enum Backend {
    Html,
    Org,
}

impl Backend {
    pub fn export(
        self,
        parsed: &org_parser::Parser,
        buf: &mut String,
        conf: ConfigOptions
    ) -> Result<(), org_exporter::ExportError> {
        match self {
            Backend::Html => org_exporter::Html::export_tree(parsed, buf, conf),
            Backend::Org => org_exporter::Org::export_tree(parsed, buf, conf),
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Backend::Html => "html",
            Backend::Org => "org",
        }
    }
}

impl Default for Backend {
    fn default() -> Self {
        Backend::Html
    }
}

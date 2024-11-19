//! Exporter for Org-Mode documents
//!
//! This crate exposes the [`Exporter`] trait, which each backend must implement.
//!
//! # Examples
//!
//! To convert an input string to HTML/Org, just invoke [`Exporter::export`]:
//!
//! ```rust
//! use org_rust_exporter as org_exporter;
//! use org_exporter::{Html, Org, Exporter, ConfigOptions};
//!
//! let html_str: String = Html::export("* Hello HTML!\n", ConfigOptions::default()).unwrap();
//! let org_str: String = Org::export("* Hello Org!\n", ConfigOptions::default()).unwrap();
//! ```
//!
//! You can also export into a buffer that implements [`fmt::Write`]:
//!
//! ```rust
//! use org_rust_exporter as org_exporter;
//! use org_exporter::{Html, Org, Exporter, ConfigOptions};
//!
//! let mut html_str = String::new();
//! let mut org_str = String::new();
//!
//! Html::export_buf("* Hello HTML!\n", &mut html_str, ConfigOptions::default());
//! Org::export_buf("* Hello Org!\n", &mut org_str, ConfigOptions::default());
//!
//! assert_eq!(html_str, r#"<h1 id="hello-html">Hello HTML!</h1>
//! "#);
//! assert_eq!(org_str, "* Hello Org!\n");
//! ```

mod html;
mod include;
mod org;
mod org_macros;
mod types;
mod utils;

pub use html::Html;
pub use org::Org;
pub use types::{ConfigOptions, ExportError, Exporter};

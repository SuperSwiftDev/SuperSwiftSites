use std::path::PathBuf;
use clap::{Parser, Subcommand};
use pretty_tree::PrettyTreePrinter;

use crate::{compile::Compiler, html::ParserMode, process::process_html_file};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Compile(CompileCli),
}

#[derive(Parser, Debug)]
pub struct CompileCli {
    /// Wrap all contents in the given template file.
    #[arg(long)]
    template: Option<PathBuf>,
    /// Array of file paths or unix style glob patterns.
    /// 
    /// The system will try to automatically resolve whether each respective input is a glob or a file path. To disable glob mode checking and treat each input as a file path see the `no_globs` flag.
    #[arg(long, num_args = 1..)]
    input: Vec<String>,
    /// The output directory.
    #[arg(long, num_args = 1..)]
    output: PathBuf,
    /// Pretty-print HTML(5) files (more pretty); default value is true.
    #[arg(long)]
    pretty_print: Option<bool>,
}

impl Cli {
    pub fn execute(self) {
        match self.command {
            Command::Compile(compile_cli) => compile_cli.execute(),
        }
    }
}

impl CompileCli {
    pub fn execute(self) {
        let compiler = Compiler {
            template_path: self.template.clone(),
            input_paths: resolve_file_path_paterns(&self.input).unwrap(),
            output_dir: self.output.clone(),
            pretty_print: self.pretty_print.unwrap_or(true),
        };
        compiler.run();
    }
}

fn resolve_file_path_paterns(patterns: &[String]) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    fn resolve_entry_as_glob(pattern: &str) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let mut results = Vec::<PathBuf>::new();
        for pattern in glob::glob(pattern)? {
            match pattern {
                Ok(path) => {
                    results.push(path);
                    continue;
                }
                Err(error) => return Err(Box::new(error)),
            }
        }
        Ok(results)
    }
    fn resolve_entry(pattern: &str) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        if let Ok(results) = resolve_entry_as_glob(pattern) {
            return Ok(results)
        }
        let path = PathBuf::from(pattern);
        return Ok(vec![path])
    }
    let mut results = Vec::<PathBuf>::new();
    for pattern in patterns {
        match resolve_entry(&pattern) {
            Ok(paths) => {
                results.extend(paths);
            }
            Err(error) => {
                return Err(error)
            }
        }
    }
    Ok(results)
}

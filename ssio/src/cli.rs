use std::path::PathBuf;
use clap::{Parser, Subcommand};
use pretty_tree::PrettyTreePrinter;

use crate::compile::Compiler;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Compile(CompileCli),
    Build(BuildCli),
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
    #[arg(long)]
    output: PathBuf,
    /// The project root.
    #[arg(long)]
    root: PathBuf,
    /// Pretty-print HTML(5) files (more pretty); default value is true.
    #[arg(long)]
    pretty_print: Option<bool>,
}

#[derive(Parser, Debug)]
pub struct BuildCli {
    #[arg(long)]
    pub manifest: PathBuf,
    /// Pretty-print HTML(5) files (more pretty); default value is true.
    #[arg(long)]
    pretty_print: Option<bool>,
}

impl Cli {
    pub fn execute(self) {
        match self.command {
            Command::Compile(compile_cli) => compile_cli.execute(),
            Command::Build(build_cli) => build_cli.execute(),
        }
    }
}

impl CompileCli {
    pub fn execute(self) {
        let input_paths = crate::path_utils::resolve_file_path_paterns(&self.input)
            .unwrap()
            .into_iter()
            .map(|path| {
                crate::compile::InputRule {
                    source: path,
                    target: None,
                }
            })
            .collect();
        let compiler = Compiler {
            project_root: self.root.clone(),
            input_paths,
            template_path: self.template.clone(),
            output_dir: self.output.clone(),
            pretty_print: self.pretty_print.unwrap_or(true),
            bundles: Default::default(),
        };
        compiler.run();
    }
}

impl BuildCli {
    pub fn execute(self) {
        let manifest_dir = self.manifest.parent().unwrap();
        let manifest = crate::manifest::load_project_manifest(&self.manifest).unwrap();
        manifest.execute(manifest_dir, self.pretty_print);
    }
}


use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::compile::Compiler;

/// The full config file
#[derive(Debug, Deserialize)]
pub struct ProjectManifest {
    #[serde(default = "default_root")]
    pub root: PathBuf,
    #[serde(default = "default_output")]
    pub output_dir: PathBuf,
    #[serde(default)]
    pub template: Option<PathBuf>,

    #[serde(default)]
    pub pretty_print: Option<bool>,

    #[serde(default)]
    pub globs: Vec<GlobRewriteRule>, 

    #[serde(default)]
    pub manual: Vec<ManualRewriteRule>,

    #[serde(default)]
    pub assets: Vec<AssetRule>,

    #[serde(default)]
    pub bundles: Vec<BundleRule>,
}

fn default_root() -> PathBuf {
    PathBuf::from(".")
}

fn default_output() -> PathBuf {
    PathBuf::from("output")
}

fn default_pretty_print() -> bool {
    true
}

/// Glob-based rewrite rules
#[derive(Debug, Deserialize)]
pub struct GlobRewriteRule {
    /// Glob pattern to match files, relative to project root
    pub pattern: String,

    /// Prefix to strip from the matched path
    #[serde(default)]
    pub strip_prefix: Option<String>,
}

/// Manual rewrite rules for specific files
#[derive(Debug, Deserialize)]
pub struct ManualRewriteRule {
    /// Input file path
    pub source: PathBuf,

    /// Desired output path
    pub target: PathBuf,
}

/// Static assets to copy into output directory
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum AssetRule {
    Glob {
        /// Glob pattern to match asset files
        pattern: String,

        /// Prefix to strip when copying assets to output
        #[serde(default)]
        strip_prefix: Option<String>,
    }
}

/// Static assets to copy into output directory
#[derive(Debug, Deserialize)]
pub struct BundleRule {
    /// Glob pattern to match asset files
    pub location: PathBuf,
}


pub fn load_project_manifest(path: impl AsRef<Path>) -> Result<ProjectManifest, Box<dyn std::error::Error>> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path)?;
    let config: ProjectManifest = toml::from_str(&text)?;
    Ok(config)
}

impl ProjectManifest {
    pub fn execute(&self, manifest_dir: impl AsRef<Path>, pretty_print: Option<bool>) {
        let manifest_dir = manifest_dir.as_ref();
        let working_dir = manifest_dir.join(&self.root);
        std::env::set_current_dir(&working_dir).unwrap();
        let bundles = self.bundles
            .iter()
            .map(|bundle| {
                crate::compile::BundleRule {
                    location: bundle.location.clone(),
                }
            })
            .collect::<Vec<_>>();
        let inputs = self.globs
            .iter()
            .flat_map(|rule| {
                crate::path_utils::resolve_file_path_paterns(&[rule.pattern.clone()])
                    .into_iter()
                    .flat_map(|x| x)
                    .map(|path| {
                        let target = rule.strip_prefix
                            .as_ref()
                            .map(|x| {
                                path.strip_prefix(x).unwrap().to_path_buf()
                            });
                        crate::compile::InputRule {
                            source: path,
                            target,
                        }
                    })
            })
            .collect::<Vec<_>>();
        let compiler = Compiler {
            project_root: working_dir,
            input_paths: inputs,
            template_path: self.template.clone(),
            output_dir: self.output_dir.clone(),
            pretty_print: self.pretty_print.unwrap_or(pretty_print.unwrap_or(true)),
            bundles,
        };
        compiler.run();
    }
}


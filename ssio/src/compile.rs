use std::{collections::{HashMap, HashSet}, path::PathBuf};

// use pretty_tree::PrettyTreePrinter;

use pretty_tree::PrettyTreePrinter;
use serde::Deserialize;

use crate::{html::ParserMode, pass::{postprocess::PostprocessEnvironment, system::{Aggregator, Dependency}}};
use crate::dependency_tracking::resolve_virtual_paths::{PathResolver, VirtualPathContext};
// use crate::process::{process_html_file, Dependency, OutputContext, SiteLink};

#[derive(Debug, Clone)]
pub struct Compiler {
    pub project_root: PathBuf,
    pub template_path: Option<PathBuf>,
    pub input_paths: Vec<InputRule>,
    pub output_dir: PathBuf,
    pub pretty_print: bool,
    pub bundles: Vec<BundleRule>,
}

/// Input file with optional rewrite rule
#[derive(Debug, Clone)]
pub struct InputRule {
    /// Input file path
    pub source: PathBuf,
    /// Desired output path
    pub target: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct BundleRule {
    pub location: PathBuf,
}

impl Compiler {
    pub fn run(&self) {
        std::fs::create_dir_all(&self.output_dir).unwrap();
        let template = self.template_path.as_ref().map(|path| {
            match crate::pass::load::load_html_file(path, ParserMode::Document, &self.project_root) {
                Ok(x) => x,
                Err(error) => {
                    eprintln!("Failed to read file: {path:?}");
                    panic!("{error}")
                }
            }
        });
        let page_contents = self.input_paths
            .clone()
            .into_iter()
            .map(|rule| {
                let source_io = crate::pass::load::load_html_file(
                    &rule.source,
                    ParserMode::fragment("div"),
                    &self.project_root
                ).unwrap();
                let baked_io = template
                    .clone()
                    .map(|template| {
                        crate::template::bake_template_content(template, source_io.clone(), true)
                    })
                    .unwrap_or_else(|| source_io.clone());
                // baked_io.value.print_pretty_tree();
                (rule.source, baked_io, rule.target)
            })
            .map(|(src_path, page, out_path)| {
                let out_path = out_path
                    .map(|out| {
                        self.output_dir.join(out)
                    })
                    .unwrap_or_else(|| {
                        let out = src_path.strip_prefix(&self.project_root).unwrap();
                        let out = out.to_path_buf();
                        self.output_dir.join(out)
                    });
                (src_path, page, out_path)
            })
            .collect::<Vec<_>>();
        // let env = page_contents
        //     .iter()
        //     .map(|(_, x, _)| x.aggregator.clone())
        //     .fold(OutputContext::default(), |acc, x| { acc.merge(x) });
        let env = Aggregator::flatten(
            page_contents
                .iter()
                .map(|(_, x, _)| x.aggregator.clone())
        );
        // println!("{env:#?}");
        let static_dependencies = env.static_dependencies
            .clone()
            .into_iter()
            .filter(|x| {
                !x.is_internal.unwrap_or(false)
            })
            .filter(|x| {
                let full_resolved_path = x.resolved_source_file_path();
                let keep = full_resolved_path.exists();
                // println!("> {keep} {full_resolved_path:?}");
                keep
            })
            .collect::<Vec<_>>();
        let site_links = env.source_dependencies
            .clone()
            .into_iter()
            .map(|x| (x.normalized_target(), x))
            .collect::<HashMap<_, _>>();
        let asset_context = AssetContext {
            project_directory: self.project_root.clone(),
            output_directory: self.output_dir.clone(),
        };
        let asset_inputs = env.static_dependencies
            .iter()
            .filter(|x| !x.is_internal.unwrap_or(false))
            .map(|x| {
                InputRule {
                    source: x.resolved_source_file_path(),
                    target: Some(x.resolved_target_file_path(&self.output_dir))
                }
            })
            .map(|x| x.clean())
            .collect::<Vec<_>>();
        // println!("{:#?}", self.bundles);
        for bundle in self.bundles.iter() {
            let source = bundle.location.clone();
            let output = self.output_dir.join(&bundle.location);
            // println!("BUNDLE: {source:?} => {output:?}");
            crate::symlink::create_relative_symlink(
                &source,
                &output
            ).unwrap();
        }
        for dependency in static_dependencies {
            let full_resolved_path = path_clean::clean(dependency.resolved_source_file_path());
            let target_path = path_clean::clean(dependency.resolved_target_file_path(&self.output_dir));
            // println!("{:#?}", self.bundles);
            if dependency.should_ignore(&self.bundles, &asset_context) {
                // println!("IGNORING: {dependency:?}: {:?} => {:?}", full_resolved_path, target_path);
                continue;
            }
            // println!("{dependency:?}: {:?} => {:?}", full_resolved_path, target_path);
            crate::symlink::create_relative_symlink(
                &full_resolved_path,
                &target_path
            ).unwrap();
        }
        let path_resolver = PathResolver {
            source_input_rules: self.input_paths.clone(),
            asset_input_rules: asset_inputs.clone(),
            project_root: self.project_root.clone(),
            output_dir: self.output_dir.clone(),
        };
        // println!("{path_resolver:#?}");
        for (src_path, page, out_path) in page_contents {
            assert!(out_path != src_path);
            assert!(out_path.starts_with(&self.output_dir));
            // let context = VirtualPathContext {
            //     output_file_path: out_path.clone(),
            //     origin_file_path: src_path.clone(),
            //     resolver: path_resolver.clone(),
            // };
            // // println!("{context:#?}");
            // let page_html = page.value.resolve_virtual_paths(&context);
            let postprocess_environment = PostprocessEnvironment {
                origin_file_path: src_path.clone(),
                output_file_path: out_path.clone(),
                resolver: path_resolver.clone(),
            };
            let finalized_html = page.value.postprocess(&postprocess_environment);

            let page_str = if self.pretty_print {
                finalized_html.pretty_html_string()
            } else {
                let doctype = "<!DOCTYPE html>";
                format!(
                    "{doctype}{}",
                    finalized_html.html_string(&Default::default()),
                )
            };
            let should_write = std::fs::read_to_string(&out_path)
                .map(|current| {
                    current !=  page_str
                })
                .unwrap_or(true);
            if should_write {
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent).unwrap();
                }
                std::fs::write(&out_path, page_str).unwrap();
            }
        }
    }
}

impl Dependency {
    fn normalized_target(&self) -> PathBuf {
        let origin_dir = self.origin.parent().unwrap();
        let normalized_target = origin_dir.join(&self.target);
        normalized_target
    }
}

impl Dependency {
    fn resolved_source_file_path(&self) -> PathBuf {
        let base = self.origin.parent().unwrap();
        let full = base.join(&self.target);
        let full = path_clean::clean(&full);
        full
    }
    fn resolved_target_file_path(&self, output_dir: impl AsRef<std::path::Path>) -> PathBuf {
        output_dir.as_ref().join(self.resolved_source_file_path())
    }
    // TODO: SUPER MESSY
    fn should_ignore(&self, bundles: &[BundleRule], asset_context: &AssetContext) -> bool {
        let project_directory = path_clean::clean(&asset_context.project_directory);
        let target = path_clean::clean(&self.target);
        let target = target
            .strip_prefix(&project_directory)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|_| target.clone());
        let target = path_clean::clean(target);
        let full_resolved_path = self.resolved_source_file_path();
        let matches_bundle = bundles
            .iter()
            .find(|bundle| {
                let bundle_location = path_clean::clean(&bundle.location);
                let bundle_path = bundle.location
                    .strip_prefix(&project_directory)
                    .unwrap_or_else(|_| &bundle_location);
                let bundle_path = path_clean::clean(bundle_path);
                let match_result_1 = target.starts_with(&bundle_path);
                let match_result_2 = full_resolved_path.starts_with(&bundle_path);
                match_result_1 || match_result_2
            });
        matches_bundle.is_some()
    }
}

impl InputRule {
    pub fn clean(self) -> Self {
        Self {
            source: path_clean::clean(&self.source),
            target: self.target.map(|target| path_clean::clean(&target)),
        }
    }
}

#[derive(Debug, Clone)]
struct AssetContext {
    pub project_directory: PathBuf,
    pub output_directory: PathBuf,
}

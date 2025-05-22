use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::html::Html;
use crate::html::Element;
use crate::compile::InputRule;

use super::data::REQUIRES_REGULAR_DEPENDENCY_TRACKING;
use super::data::TAG_MAY_REQUIRE_DEPENDENCY_TRACKING;
use super::data::REQUIRES_SRC_SET_DEPENDENCY_TRACKING;
use super::data::SrcsetCandidate;

#[derive(Debug, Clone)]
pub struct VirtualPathContext<'a> {
    /// The source file path this HTML node came from
    pub origin_file_path: &'a Path,
    /// The file path where the output will be written
    pub output_file_path: &'a Path,
    /// The virtual link resolver
    pub resolver: &'a PathResolver,
}

// === Rewriting Paths in Html ===

pub fn resolve_virtual_paths(tag: &str, attributes: &mut HashMap<String, String>, context: &VirtualPathContext) {
    let tag = tag.to_lowercase();
    for (key, value) in attributes.iter_mut() {
        let key = key.to_lowercase();
        if REQUIRES_REGULAR_DEPENDENCY_TRACKING.contains(&(&tag, &key)) {
            rewrite_path(
                value,
                &context.origin_file_path,
                &context.output_file_path,
                &context.resolver,
            );
        }
        else if REQUIRES_SRC_SET_DEPENDENCY_TRACKING.contains(&(&tag, &key)) {
            let source_sets = SrcsetCandidate::parse_srcset(value)
                .into_iter()
                .map(|SrcsetCandidate { mut url, descriptor }| {
                    rewrite_path(
                        &mut url,
                        &context.origin_file_path,
                        &context.output_file_path,
                        &context.resolver,
                    );
                    SrcsetCandidate {
                        url,
                        descriptor: descriptor,
                    }
                })
                .collect::<Vec<_>>();
            let rewritten_source_sets = SrcsetCandidate::format_srcset(&source_sets);
            *value = rewritten_source_sets;
        }
    }
}

fn rewrite_path(
    href: &mut String,
    origin_file: &Path,
    output_file: &Path,
    resolver: &PathResolver,
) {
    if crate::path_utils::is_external_url(href) {
        return;
    }

    let raw = href.as_str();

    let resolved_target = if let Some(clean) = raw.strip_prefix("@/") {
        // 🌍 Treat "@/..." as relative to project root
        path_clean::clean(resolver.project_root.join(clean))
    } else {
        // 📄 Otherwise, resolve relative to origin file
        path_clean::clean(origin_file.parent().unwrap().join(raw))
    };

    if let Some(dest_output_path) = resolver.resolve_output_path_resolved(&resolved_target) {
        if let Some(relative) = pathdiff::diff_paths(&dest_output_path, output_file.parent().unwrap()) {
            *href = relative.to_string_lossy().to_string();
        } else {
            eprintln!("⚠️  Failed diff_paths from {dest_output_path:?} to {:?}", output_file);
        }
    } else {
        eprintln!(
            "⚠️  Could not resolve output path for target {:?} (normalized: {:?}) in {:?}",
            href,
            resolved_target,
            origin_file,
        );
    }
}


#[derive(Debug, Clone)]
pub struct PathResolver {
    /// HTML input files.
    pub source_input_rules: Vec<InputRule>,
    /// Static assets.
    pub asset_input_rules: Vec<InputRule>,
    pub project_root: PathBuf,
    pub output_dir: PathBuf,
}

impl PathResolver {
    fn resolve_output_path_resolved(&self, resolved_target: &Path) -> Option<PathBuf> {
        self.try_resolve_input_rule(resolved_target)
            .or_else(|| {
                self.try_resolve_asset_dep(resolved_target)
            })
    }
    fn try_resolve_input_rule(&self, resolved_target: &Path) -> Option<PathBuf> {
        Self::lookup_input_rule(resolved_target, &self.source_input_rules).map(|rule| {
            let output_rel = rule
                .target
                .clone()
                .unwrap_or_else(|| rule.source.strip_prefix(&self.project_root).unwrap().to_path_buf());
            self.output_dir.join(output_rel)
        })
    }
    fn try_resolve_asset_dep(&self, resolved_target: &Path) -> Option<PathBuf> {
        // self.resolve_target_path(resolved_target, &self.asset_input_rules)
        Self::lookup_input_rule(resolved_target, &self.asset_input_rules).map(|rule| {
            rule
                .target
                .clone()
                .unwrap_or_else(|| rule.source.strip_prefix(&self.project_root).unwrap().to_path_buf())
        })
    }
    fn lookup_input_rule(resolved_target: &Path, rules: &[InputRule]) -> Option<InputRule> {
        rules
            .iter()
            .find(|rule| {
                path_clean::clean(&rule.source) == path_clean::clean(resolved_target)
            })
            .map(ToOwned::to_owned)
    }
}


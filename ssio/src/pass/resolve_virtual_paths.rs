use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{
    compile::InputRule,
    html::{Element, Html},
};

#[derive(Debug, Clone)]
pub struct VirtualPathContext {
    /// The file path where the output will be written
    pub output_file_path: PathBuf,
    /// The source file path this HTML node came from
    pub origin_file_path: PathBuf,
    /// The virtual link resolver
    pub resolver: PathResolver,
}

// === Rewriting Paths in Html ===

impl Html {
    pub fn resolve_virtual_paths(self, context: &VirtualPathContext) -> Self {
        match self {
            Self::Element(element) => Self::Element(element.resolve_virtual_paths(context)),
            Self::Text(text) => Self::Text(text),
            Self::Fragment(nodes) => Self::Fragment(
                nodes
                    .into_iter()
                    .map(|x| x.resolve_virtual_paths(context))
                    .collect(),
            ),
        }
    }
}

impl Element {
    pub fn resolve_virtual_paths(self, context: &VirtualPathContext) -> Self {
        let children = self
            .children
            .into_iter()
            .map(|x| x.resolve_virtual_paths(context))
            .collect();

        let attrs = process_attributes(self.attrs, context);
        Self {
            attrs,
            children,
            tag: self.tag,
        }
    }
}

fn process_attributes(
    mut attributes: HashMap<String, String>,
    context: &VirtualPathContext,
) -> HashMap<String, String> {
    for key in ["href", "src"] {
        if let Some(val) = attributes.get_mut(key) {
            rewrite_path(
                val,
                &context.origin_file_path,
                &context.output_file_path,
                &context.resolver,
            );
        }
    }
    attributes
}

pub fn rewrite_path(
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
            href, resolved_target, origin_file
        );
    }
}


#[derive(Debug, Clone)]
pub struct PathResolver {
    pub input_rules: Vec<InputRule>, // HTML input files
    pub asset_inputs: Vec<PathBuf>,  // Static assets
    pub project_root: PathBuf,
    pub output_dir: PathBuf,
}

impl PathResolver {
    pub fn resolve_output_path_resolved(&self, resolved_target: &Path) -> Option<PathBuf> {
        // Site input files
        if let Some(rule) = self.input_rules.iter().find(|rule| {
            path_clean::clean(&rule.source) == path_clean::clean(resolved_target)
        }) {
            let output_rel = rule
                .target
                .clone()
                .unwrap_or_else(|| rule.source.strip_prefix(&self.project_root).unwrap().to_path_buf());

            return Some(self.output_dir.join(output_rel));
        }

        // Asset files
        if let Some(asset) = self.asset_inputs.iter().find(|asset| {
            let left = path_clean::clean(asset);
            let right = path_clean::clean(resolved_target);
            let is_match = left == right;
            return is_match
        }) {
            let asset_path = asset
                .strip_prefix(&self.project_root)
                .map(ToOwned::to_owned)
                .unwrap_or_else(|_| asset.to_owned());
            return Some(self.output_dir.join(asset_path));
        }

        None
    }
}


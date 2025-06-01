use std::collections::HashMap;
use std::path::PathBuf;

use crate::html::Html;
use crate::html::Element;
use crate::html_pass::system::State;
use crate::html_pass::system::Aggregator;
use crate::html_pass::system::Dependency;
use crate::html_pass::system::Scope;

use super::data::REQUIRES_REGULAR_DEPENDENCY_TRACKING;
use super::data::REQUIRES_SRC_SET_DEPENDENCY_TRACKING;
use super::data::TAG_MAY_REQUIRE_DEPENDENCY_TRACKING;
use super::data::REQUIRES_DYNAMIC_SITE_LINK_DEPENDENCY_TRACKING;
use super::data::SrcsetCandidate;

pub fn virtualize_and_register_local_paths(
    tag: &str,
    attributes: &mut HashMap<String, String>,
    scope: &Scope,
    aggregator: &mut Aggregator
) {
    let tag = tag.to_lowercase();
    if !TAG_MAY_REQUIRE_DEPENDENCY_TRACKING.contains(tag.as_str()) {
        return 
    }
    // - -
    for (key, value) in attributes.iter_mut() {
        let key = key.to_lowercase();
        if let Some(rewritten) = try_to_virtual_path_value(&tag, &key, &value, scope) {
            let source = path_clean::clean(scope.source_path.clone());
            let target = path_clean::clean(PathBuf::from(&value));
            if REQUIRES_DYNAMIC_SITE_LINK_DEPENDENCY_TRACKING.contains(&(&tag, &key)) {
                aggregator.source_dependencies.insert(Dependency { origin: source, target, is_internal: None });
            } else {
                let is_internal = tag == "include";
                aggregator.static_dependencies.insert(Dependency { origin: source, target, is_internal: Some(is_internal) });
            }
            *value = rewritten;
        }
    }
}

fn try_to_virtual_path_value(tag: &str, key: &str, value: &str, scope: &Scope) -> Option<String> {
    // REGAULR
    if REQUIRES_REGULAR_DEPENDENCY_TRACKING.contains(&(&tag, &key)) {
        let target = PathBuf::from(value);
        let virtual_src = crate::path_utils::normalize_virtual_path(
            value,
            &scope.source_path,
            &scope.project_root,
        );
        return Some(virtual_src)
    }
    // SPECIAL
    else if REQUIRES_SRC_SET_DEPENDENCY_TRACKING.contains(&(&tag, &key)) {
        let source_sets = SrcsetCandidate::parse_srcset(value)
            .into_iter()
            .map(|SrcsetCandidate { url, descriptor }| {
                let virtual_src = crate::path_utils::normalize_virtual_path(
                    &url,
                    &scope.source_path,
                    &scope.project_root,
                );
                SrcsetCandidate {
                    url: virtual_src,
                    descriptor,
                }
            })
            .collect::<Vec<_>>();
        let rewritten_source_sets = SrcsetCandidate::format_srcset(&source_sets);
        return Some(rewritten_source_sets)
    }
    None
}





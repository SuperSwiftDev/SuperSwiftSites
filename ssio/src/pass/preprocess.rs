use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{html::{Html, ParserMode}, pass::system::Dependency};
use crate::html::Element;
use crate::compile::InputRule;
use crate::pass::system::Scope;
use crate::pass::system::State;
use crate::pass::system::Aggregator;
use crate::dependency_tracking::virtualize_local_paths::virtualize_and_register_local_paths;

impl Html {
    pub fn preprocess(self, scope: &Scope) -> State<Self> {
        match self {
            // Self::Element(element) => element.preprocess(scope).and_then(|element| {
            //     if element.tag == "include" || element.tag == "INCLUDE" {
            //         let Element { tag, attrs, children } = element;
            //         return process_include_tag(attrs, children, scope)
            //     }
            //     State::wrap(Self::Element(element))
            // }),
            Self::Element(element) => element.preprocess(scope),
            Self::Text(text) => State::wrap(Self::Text(text)),
            Self::Fragment(nodes) => preprocess_fragment(nodes, scope).map(Self::Fragment),
        }
    }
}

impl Element {
    pub fn preprocess(self, scope: &Scope) -> State<Html> {
        let Element { tag, mut attrs, children } = self;
        if tag == "include" || tag == "INCLUDE" {
            return process_include_tag(attrs, children, scope)
        }
        preprocess_fragment(children, scope).map_with(|children, ctx| {
            virtualize_and_register_local_paths(&tag, &mut attrs, scope, ctx);
            Html::Element(Element {
                tag: tag,
                attrs: attrs,
                children: children
            })
        })
    }
}

fn preprocess_fragment(nodes: Vec<Html>, scope: &Scope) -> State<Vec<Html>> {
    let nodes_len = nodes.len();
    let nodes = nodes
        .into_iter()
        .map(|node| node.preprocess(scope));
    State::flatten(nodes, Some(nodes_len))
}

fn process_include_tag(
    attrs: HashMap<String, String>,
    children: Vec<Html>,
    scope: &Scope,
) -> State<Html> {
    let content = preprocess_fragment(children, scope).map(|children| {
        Html::Fragment(children)
    });
    if let Some(src_value) = attrs.get("src").cloned() {
        let src_value = PathBuf::from(path_clean::clean(src_value));
        let resolved_path = scope.source_dir().join(&src_value);
        let template = super::load::load_html_file(
            &resolved_path,
            ParserMode::fragment("div"),
            &scope.project_root,
        ).unwrap(); // TODO: SAFE HANDLING
        // let template = process_html_file(
        //     &resolved_path,
        //     ParserMode::fragment("div"),
        //     &input.project_root,
        // ).unwrap();
        let mut baked_node = crate::template::bake_template_content(template, content, false);
        let dependency = Dependency {
            origin: path_clean::clean(&scope.source_path),
            target: path_clean::clean(src_value),
            is_internal: Some(true),
        };
        baked_node.aggregator.static_dependencies.insert(dependency); // TODO: NOT A STATIC DEPENDENCY
        // println!("resolved_path: {resolved_path:?}");
        // println!("resolved_path: {resolved_path:?}:{}", baked_node.value.to_pretty_tree());
        return baked_node
    }
    eprintln!("⚠️ FAILED TO RESOLVE INCLUDE IN FILE: {:?}", scope.source_path);
    content
}

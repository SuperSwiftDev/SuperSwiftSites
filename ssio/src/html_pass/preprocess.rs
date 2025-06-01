use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{html::{Html, ParserMode}, html_pass::system::Dependency};
use crate::html::Element;
use crate::compile::InputRule;
use crate::html_pass::system::Scope;
use crate::html_pass::system::State;
use crate::html_pass::system::Aggregator;
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
        match &tag.to_lowercase()[..] {
            "include" => {
                return process_include_tag(attrs, children, scope)
            }
            "style" => {
                return process_style_tag(attrs, children, scope)
            }
            _ => ()
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
        let resolved_path = scope.source_dir().join(&src_value);
        // - DEPENDENCY -
        let dependency = Dependency {
            origin: path_clean::clean(&scope.source_path),
            target: path_clean::clean(src_value),
            is_internal: Some(true),
        };
        // - LOAD -
        let template = super::load::load_html_file(
            &resolved_path,
            ParserMode::fragment("div"),
            &scope.project_root,
        );
        let template = match template {
            Ok(x) => x,
            Err(error) => {
                let source_path = scope.source_path.as_path();
                if let Some(error) = error.downcast_ref::<std::io::Error>() {
                    eprintln!("⚠️ {source_path:?} file not found: {resolved_path:?}");
                } else {
                    eprintln!("⚠️ {source_path:?}: {error}");
                }
                return State::wrap(Html::Fragment(Vec::default()))
            }
        };
        let mut baked_node = crate::template::bake_template_content(template, content, false);
        baked_node.aggregator.static_dependencies.insert(dependency); // TODO: NOT A STATIC DEPENDENCY
        return baked_node
    }
    eprintln!("⚠️ FAILED TO RESOLVE INCLUDE IN FILE: {:?}", scope.source_path);
    content
}

fn process_style_tag(
    mut attrs: HashMap<String, String>,
    children: Vec<Html>,
    scope: &Scope,
) -> State<Html> {
    preprocess_fragment(children, scope).map_with(|children, ctx| {
        virtualize_and_register_local_paths("style", &mut attrs, scope, ctx);
        let source_code = Html::Fragment(children).to_text().unwrap();
        let source_code = crate::css_process::pre_process(&source_code, scope, ctx);
        let children = vec![
            Html::Text(source_code),
        ];
        Html::Element(Element {
            tag: String::from("style"),
            attrs: attrs,
            children,
        })
    })
}

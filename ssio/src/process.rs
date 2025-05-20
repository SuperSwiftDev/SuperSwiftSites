use std::{collections::{HashMap, HashSet}, path::PathBuf};

use pretty_tree::PrettyTreePrinter;

use crate::html::{Element, Html, ParserMode};

// #[derive(Debug, Clone)]
// pub struct ProjectContext {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputContext {
    pub project_root: PathBuf,
    pub source_path: PathBuf,
}

impl InputContext {
    pub fn source_dir(&self) -> PathBuf {
        self.source_path.parent().unwrap().to_path_buf()
    }
}

#[derive(Debug, Clone, Default)]
pub struct OutputContext {
    pub dependencies: HashSet<Dependency>,
    pub site_link: HashSet<SiteLink>,
}

impl OutputContext {
    pub fn io<Value>(self, value: Value) -> IO<Value> {
        IO { context: self, value }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Dependency {
    pub origin: PathBuf,
    pub target: PathBuf,
    pub internal: bool,
}

impl OutputContext {
    pub fn union(left: Self, right: Self) -> Self {
        OutputContext {
            dependencies: left.dependencies.union(&right.dependencies).cloned().collect(),
            site_link: left.site_link.union(&right.site_link).cloned().collect(),
        }
    }
    pub fn merge(self, other: Self) -> Self {
        Self::union(self, other)
    }
    pub fn include(&mut self, other: Self) {
        self.dependencies.extend(other.dependencies);
        self.site_link.extend(other.site_link);
    }
}

#[derive(Debug, Clone)]
pub struct IO<Value> {
    pub context: OutputContext,
    pub value: Value,
}

impl<Value> IO<Value> {
    pub fn map<Result>(self, apply: impl FnOnce(Value) -> Result) -> IO<Result> {
        IO { context: self.context, value: apply(self.value) }
    }
    pub fn map_with<Result>(self, apply: impl FnOnce(Value, &mut OutputContext) -> Result) -> IO<Result> {
        let mut context = self.context;
        let result_value = apply(self.value, &mut context);
        IO { context: context, value: result_value }
    }
    pub fn wrap(value: Value) -> Self {
        Self { context: Default::default(), value: value }
    }
    pub fn flatten(items: impl IntoIterator<Item=IO<Value>>) -> IO<Vec<Value>> {
        items
            .into_iter()
            .fold(IO::<Vec<Value>>::default(), |mut acc, item| {
                let IO { context, value } = item;
                acc.context.include(context);
                acc.value.push(value);
                acc
            })
    }
}

impl<Value> Default for IO<Vec<Value>> {
    fn default() -> Self {
        IO { context: OutputContext::default(), value: Vec::default() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SiteLink {
    pub origin: PathBuf,
    pub target: PathBuf,
}

pub fn process_html_file(
    file_path: impl AsRef<std::path::Path>,
    parser_mode: ParserMode,
    project_root: impl AsRef<std::path::Path>,
) -> Result<IO<Html>, Box<dyn std::error::Error>> {
    let file_path = file_path.as_ref().to_path_buf();
    let source = std::fs::read_to_string(&file_path)?;
    let source_tree = Html::parse(&source, parser_mode);
    let input_ctx = InputContext {
        source_path: file_path,
        project_root: project_root.as_ref().to_path_buf(),
    };
    Ok(process_html_tree(source_tree, &input_ctx))
}

fn process_html_tree(input: Html, input_ctx: &InputContext) -> IO<Html> {
    input.process(input_ctx)
}


impl Html {
    fn process(self, input: &InputContext) -> IO<Self> {
        match self {
            Self::Element(element) => element.process(input),
            Self::Fragment(nodes) => {
                process_fragment(nodes, input).map(Html::Fragment)
            }
            Self::Text(text) => IO::wrap(Self::Text(text)),
        }
    }
}

impl Element {
    fn process(self, input: &InputContext) -> IO<Html> {
        if let Some(tag_type) = TagType::from(&self.tag) {
            return match tag_type {
                TagType::A => process_a_tag(self.attrs, self.children, input),
                TagType::Img => process_img_tag(self.attrs, self.children, input),
                TagType::Include => process_include_tag(self.attrs, self.children, input),
                TagType::Link => process_link_tag(self.attrs, self.children, input),
            }
        }
        process_fragment(self.children, input).map(|children| {
            Html::Element(Element {
                tag: self.tag,
                attrs: self.attrs,
                children: children
            })
        })
    }
}

fn process_fragment(nodes: Vec<Html>, input: &InputContext) -> IO<Vec<Html>> {
    nodes
        .into_iter()
        .map(|child| child.process(input))
        .fold(IO::<Vec<Html>>::default(), |mut acc, item| {
            let IO { context, value } = item;
            acc.context.include(context);
            acc.value.push(value);
            acc
        })
}

enum TagType {
    A,
    Img,
    Include,
    Link,
}

impl TagType {
    fn from(tag: &str) -> Option<Self> {
        match tag {
            "a" => Some(Self::A),
            "img" => Some(Self::Img),
            "include" => Some(Self::Include),
            "link" => Some(Self::Link),
            _ => None,
        }
    }
}

fn process_img_tag(
    mut attrs: HashMap<String, String>,
    children: Vec<Html>,
    input: &InputContext,
) -> IO<Html> {
    process_fragment(children, input).map_with(|children, ctx| {
        if let Some(src_value) = attrs.get("src") {
            let source = input.source_path.clone();
            let target = PathBuf::from(src_value);
            ctx.dependencies.insert(Dependency { origin: source, target, internal: false });
            // - -
            let virtual_src = crate::path_utils::normalize_virtual_path(
                &src_value,
                &input.source_path,
                &input.project_root,
            );
            attrs.insert(String::from("src"), virtual_src);
        }
        Html::Element(Element { tag: String::from("img"), attrs: attrs, children: children })
    })
}

fn process_link_tag(
    mut attrs: HashMap<String, String>,
    children: Vec<Html>,
    input: &InputContext,
) -> IO<Html> {
    process_fragment(children, input).map_with(|children, ctx| {
        let mut src_path: Option<String> = None;
        if let Some(rel_value) = attrs.get("rel") {
            if rel_value == "stylesheet" {
                if let Some(href_value) = attrs.get("href") {
                    if !crate::path_utils::is_external_url(&href_value) {
                        src_path = Some(href_value.clone());
                    }
                }
            }
        }
        if let Some(href_value) = src_path {
            let origin = input.source_path.clone();
            let target = PathBuf::from(href_value.clone());
            let new_dependency = Dependency { origin, target, internal: false };
            ctx.dependencies.insert(new_dependency);
            // - -
            let virtual_href = crate::path_utils::normalize_virtual_path(
                &href_value,
                &input.source_path,
                &input.project_root,
            );
            attrs.insert(String::from("href"), virtual_href);
        }
        Html::Element(Element { tag: String::from("link"), attrs: attrs, children: children })
    })
}

fn process_a_tag(
    mut attrs: HashMap<String, String>,
    children: Vec<Html>,
    input: &InputContext,
) -> IO<Html> {
    process_fragment(children, input).map_with(|children, ctx| {
        if let Some(from_value) = attrs.get("href") {
            let from_path = PathBuf::from(from_value.clone());
            let origin = path_clean::clean(&input.source_path);
            let target = path_clean::clean(&from_path);
            let site_link = SiteLink {
                origin,
                target,
            };
            ctx.site_link.insert(site_link);
            // - -
            let virtual_href = crate::path_utils::normalize_virtual_path(
                &from_value,
                &input.source_path,
                &input.project_root,
            );
            attrs.insert(String::from("href"), virtual_href);
        }
        Html::Element(Element { tag: String::from("a"), attrs: attrs, children: children })
    })
}

fn process_include_tag(
    attrs: HashMap<String, String>,
    children: Vec<Html>,
    input: &InputContext,
) -> IO<Html> {
    if let Some(src_value) = attrs.get("src").cloned() {
        let src_value = PathBuf::from(path_clean::clean(src_value));
        let source_io = IO::wrap(Html::Fragment(children));
        let resolved_path = input.source_dir().join(&src_value);
        // println!("resolved_path: {resolved_path:?}");
        let template = process_html_file(
            &resolved_path,
            ParserMode::fragment("div"),
            &input.project_root,
        )
            .unwrap();
            // .map(|template| {
            //     let path_rewrite = crate::pass::path_rewrite::PathRewrite {
            //         from_original: path_clean::clean(resolved_path),
            //         to_target: path_clean::clean(input.source_path.clone()),
            //     };
            //     template.apply_path_rewrite(&path_rewrite)
            // });
        let mut baked_node = crate::template::bake_template_content(template, source_io.clone(), false);
        let dependency = Dependency {
            origin: path_clean::clean(&input.source_path),
            target: path_clean::clean(src_value),
            internal: true,
        };
        baked_node.context.dependencies.insert(dependency);
        return baked_node
    }
    process_fragment(children, input).map(|children| {
        Html::Fragment(children)
    })
}

// /// Determine if an href points to a local file.
// fn is_local_href(href: &str) -> bool {
//     // Trim whitespace and decode HTML entities like &amp;
//     let decoded = html_escape::decode_html_entities(href.trim());

//     // Reject URLs that are clearly external
//     let lowered = decoded.to_ascii_lowercase();
//     !(lowered.starts_with("http://")
//         || lowered.starts_with("https://")
//         || lowered.starts_with("//"))
// }

fn ensure_absolute_path_prefix(route: &str) -> String {
    let route = route.strip_prefix("/").map(ToOwned::to_owned).unwrap_or_else(|| route.to_string());
    let route = format!("/{route}");
    route
}

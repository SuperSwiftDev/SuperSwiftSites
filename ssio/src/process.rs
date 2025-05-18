use std::{collections::{HashMap, HashSet}, path::PathBuf};

use crate::html::{Element, Html, ParserMode};

// #[derive(Debug, Clone)]
// pub struct ProjectContext {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputContext {
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
    pub routes: HashSet<NavLinkRoute>,
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
            routes: left.routes.union(&right.routes).cloned().collect(),
        }
    }
    pub fn merge(self, other: Self) -> Self {
        Self::union(self, other)
    }
    pub fn include(&mut self, other: Self) {
        self.dependencies.extend(other.dependencies);
        self.routes.extend(other.routes);
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
pub struct NavLinkRoute {
    pub origin: PathBuf,
    pub internal: PathBuf,
    pub external: String,
}

pub fn process_html_file(file_path: impl AsRef<std::path::Path>, parser_mode: ParserMode) -> Result<IO<Html>, Box<dyn std::error::Error>> {
    let file_path = file_path.as_ref().to_path_buf();
    let source = std::fs::read_to_string(&file_path)?;
    let source_tree = Html::parse(&source, parser_mode);
    let input_ctx = InputContext {
        source_path: file_path,
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
                TagType::Img => process_img_tag(self.attrs, self.children, input),
                TagType::NavLink => process_nav_link_tag(self.attrs, self.children, input),
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
    Img,
    NavLink,
    Include,
    Link,
}

impl TagType {
    fn from(tag: &str) -> Option<Self> {
        match tag {
            "img" => Some(Self::Img),
            "nav-link" => Some(Self::NavLink),
            "include" => Some(Self::Include),
            "link" => Some(Self::Link),
            _ => None,
        }
    }
}

fn process_img_tag(
    attrs: HashMap<String, String>,
    children: Vec<Html>,
    input: &InputContext,
) -> IO<Html> {
    process_fragment(children, input).map_with(|children, ctx| {
        if let Some(src_value) = attrs.get("src") {
            let source = input.source_path.clone();
            let target = PathBuf::from(src_value);
            ctx.dependencies.insert(Dependency { origin: source, target, internal: false });
        }
        Html::Element(Element { tag: String::from("img"), attrs: attrs, children: children })
    })
}

fn process_link_tag(
    attrs: HashMap<String, String>,
    children: Vec<Html>,
    input: &InputContext,
) -> IO<Html> {
    process_fragment(children, input).map_with(|children, ctx| {
        let mut src_path: Option<String> = None;
        if let Some(rel_value) = attrs.get("rel") {
            if rel_value == "stylesheet" {
                if let Some(href_value) = attrs.get("href") {
                    if is_local_href(&href_value) {
                        src_path = Some(href_value.clone());
                    }
                }
            }
        }
        if let Some(src_value) = src_path {
            let source = input.source_path.clone();
            let target = PathBuf::from(src_value);
            let new_dependency = Dependency { origin: source, target, internal: false };
            ctx.dependencies.insert(new_dependency);
        }
        Html::Element(Element { tag: String::from("link"), attrs: attrs, children: children })
    })
}

fn process_nav_link_tag(
    mut attrs: HashMap<String, String>,
    children: Vec<Html>,
    input: &InputContext,
) -> IO<Html> {
    process_fragment(children, input).map_with(|children, ctx| {
        let mut rewrite: Option<(PathBuf, String)> = None;
        if let Some(from_value) = attrs.get("from") {
            let from_path = PathBuf::from(from_value);
            if let Some(as_value) = attrs.get("as") {
                rewrite = Some((from_path, as_value.to_owned()));
                let _ = attrs.remove("from");
                let _ = attrs.remove("as");
            }
        }
        if let Some((from, to)) = rewrite {
            attrs.insert(String::from("href"), to.clone());
            let route = NavLinkRoute {
                origin: input.source_path.clone(),
                internal: from,
                external: to,
            };
            ctx.routes.insert(route);
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
        let src_value = PathBuf::from(src_value);
        let source_io = IO::wrap(Html::Fragment(children));
        let resolved_path = input.source_dir().join(&src_value);
        let template = process_html_file(&resolved_path, ParserMode::fragment("div")).unwrap();
        let mut baked_node = crate::template::bake_template_content(template, source_io.clone(), false);
        let dependency = Dependency {
            origin: input.source_path.clone(),
            target: src_value,
            internal: true,
        };
        baked_node.context.dependencies.insert(dependency);
        return baked_node
    }
    process_fragment(children, input).map(|children| {
        Html::Fragment(children)
    })
}

/// Determine if an href points to a local file.
fn is_local_href(href: &str) -> bool {
    // Trim whitespace and decode HTML entities like &amp;
    let decoded = html_escape::decode_html_entities(href.trim());

    // Reject URLs that are clearly external
    let lowered = decoded.to_ascii_lowercase();
    !(lowered.starts_with("http://")
        || lowered.starts_with("https://")
        || lowered.starts_with("//"))
}


use std::collections::HashMap;
use pretty_tree::{PrettyTreePrinter, ToPrettyTree};

// ————————————————————————————————————————————————————————————————————————————
// DATA MODEL
// ————————————————————————————————————————————————————————————————————————————

#[derive(Debug, Clone)]
pub enum Html {
    Element(Element),
    Text(String),
    Fragment(Vec<Html>),
}

#[derive(Debug, Clone)]
pub struct Element {
    pub tag: String,
    pub attrs: HashMap<String, String>,
    pub children: Vec<Html>,
}

impl Html {
    pub fn parse(source: &str, mode: ParserMode) -> Html {
        let result = match mode {
            ParserMode::Document => Self::parse_document(source),
            ParserMode::Fragment { context } => Self::parse_fragment(source, &context)
        };
        result
    }
    fn parse_fragment(source: &str, context: &str) -> Html {
        crate::html_parser2::parse_html_fragment(source, context).normalize()
        // let _ = context;
        // Self::parse_document(source)
    }
    fn parse_document(source: &str) -> Html {
        crate::html_parser2::parse_html_document(source).normalize()
        // let result = crate::html_parser::parse_html_str(source);
        // if result.payload.len() == 1 {
        //     return result.payload.get(0).unwrap().clone()
        // }
        // Html::Fragment(result.payload)
    }
    pub fn to_text(&self) -> Result<String, ()> {
        match self {
            Self::Element(x) => x.to_text(),
            Self::Text(x) => Ok(x.to_owned()),
            Self::Fragment(xs) => fragment_to_text(xs),
        }
    }
}

impl Element {
    pub fn has_tag(&self, tag: impl AsRef<str>) -> bool {
        self.tag.to_lowercase() == tag.as_ref().to_lowercase()
    }
    pub fn to_text(&self) -> Result<String, ()> {
        fragment_to_text(&self.children)
    }
}

fn fragment_to_text(nodes: &[Html]) -> Result<String, ()> {
    let results = nodes
        .into_iter()
        .map(|x| x.to_text())
        .collect::<Vec<_>>();
    let mut output = String::default();
    for result in results {
        match result {
            Ok(txt) => {
                output.push_str(&txt);
            }
            Err(_) => {
                return Err(())
            }
        }
    }
    Ok(output)
}

// ————————————————————————————————————————————————————————————————————————————
// CONVERSTION
// ————————————————————————————————————————————————————————————————————————————
impl crate::html_parser2::Html {
    pub fn normalize(self) -> Html {
        match self {
            crate::html_parser2::Html::Element(element) => element.normalize(),
            crate::html_parser2::Html::Fragment(nodes) => {
                let nodes = nodes.into_iter().map(|x| x.normalize()).collect();
                Html::Fragment(nodes)
            },
            crate::html_parser2::Html::Text(text) => Html::Text(text),
        }
    }
}

impl crate::html_parser2::Element {
    pub fn normalize(self) -> Html {
        let children = self.children
            .into_iter()
            .map(|x| x.normalize())
            .collect();
        Html::Element(Element { tag: self.tag, attrs: self.attrs, children: children })
    }
}

// ————————————————————————————————————————————————————————————————————————————
// DEBUG
// ————————————————————————————————————————————————————————————————————————————
impl ToPrettyTree for Html {
    fn to_pretty_tree(&self) -> pretty_tree::PrettyTree {
        match self {
            Self::Element(element) => element.to_pretty_tree(),
            Self::Text(text) => {
                pretty_tree::PrettyTree::str(text)
            }
            Self::Fragment(nodes) => {
                let nodes = nodes
                    .iter()
                    .map(|x| x.to_pretty_tree())
                    .collect::<Vec<_>>();
                pretty_tree::PrettyTree::fragment(nodes)
            }
        }
    }
}

impl ToPrettyTree for Element {
    fn to_pretty_tree(&self) -> pretty_tree::PrettyTree {
        let attrs = self.attrs
            .iter()
            .map(|(key, value)| {
                format!("{key} = {value:?}")
            })
            .collect::<Vec<_>>()
            .join(" ");
        let mut children = self.children
            .iter()
            .map(|x| x.to_pretty_tree())
            .collect::<Vec<_>>();
        if attrs.is_empty() {
            let label = format!("{}:", self.tag);
            return pretty_tree::PrettyTree::branch_of(label, &children)
        }
        if attrs.len() > 80 {
            let label = format!("{}:", self.tag);
            let mut content = vec![
                pretty_tree::PrettyTree::leaf(format!("[ {attrs} ]")),
            ];
            content.append(&mut children);
            return pretty_tree::PrettyTree::branch_of(label, &content)
        }
        let label = format!("{} [ {attrs} ]:", self.tag);
        return pretty_tree::PrettyTree::branch_of(label, &children)
    }
}

// ————————————————————————————————————————————————————————————————————————————
// HTML API UTILITIES
// ————————————————————————————————————————————————————————————————————————————

impl Html {
    pub fn as_element(&self) -> Option<&Element> {
        match self {
            Self::Element(element) => Some(element),
            _ => None,
        }
    }
    pub fn is_inline_node(&self) -> bool {
        self.as_element().map(|x| x.is_inline_node()).unwrap_or(false)
    }
    pub fn is_header_tag(&self) -> bool {
        self.as_element().map(|x| x.is_header_tag()).unwrap_or(false)
    }
}

impl Element {
    pub fn is_inline_node(&self) -> bool {
        is_inline_tag(&self.tag)
    }
    pub fn is_header_tag(&self) -> bool {
        is_header_tag(&self.tag)
    }
}

/// Returns true if tag is a known inline element based on HTML5 content model.
pub fn is_inline_tag(tag: &str) -> bool {
    match tag.to_ascii_lowercase().as_str() {
        // ————————————————
        // Phrasing Content (Inline Textual)
        // ————————————————
        "a" | "abbr" | "b" | "bdi" | "bdo" | "br" | "cite" | "code" | "data" |
        "dfn" | "em" | "i" | "kbd" | "mark" | "q" | "rp" | "rt" | "ruby" |
        "s" | "samp" | "small" | "span" | "strong" | "sub" | "sup" | "time" |
        "u" | "var" | "wbr" |

        // ————————————————
        // Embedded Content
        // ————————————————
        "audio" | "canvas" | "embed" | "iframe" | "img" | "math" |
        "object" | "picture" | "svg" | "video" |

        // ————————————————
        // Interactive Content
        // ————————————————
        "button" | "input" | "label" | "select" | "textarea" |

        // ————————————————
        // Script-supporting / Transparent
        // ————————————————
        "script" | "noscript" | "template" | "slot" | "output" => true,

        _ => false,
    }
}

pub fn is_header_tag(tag: &str) -> bool {
    tag == "h1" ||
    tag == "h2" ||
    tag == "h3" ||
    tag == "h4" ||
    tag == "h5" ||
    tag == "h6"
}

pub fn is_void_tag(tag: &str) -> bool {
    tag == "area" ||
    tag == "base" ||
    tag == "br" ||
    tag == "col" ||
    tag == "embed" ||
    tag == "hr" ||
    tag == "img" ||
    tag == "input" ||
    tag == "link" ||
    tag == "meta" ||
    tag == "source" ||
    tag == "track" ||
    tag == "wbr"
}

// ————————————————————————————————————————————————————————————————————————————
// PARSER
// ————————————————————————————————————————————————————————————————————————————

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParserMode {
    Document, Fragment { context: String }
}

impl ParserMode {
    pub fn fragment(context: impl AsRef<str>) -> Self {
        Self::Fragment { context: context.as_ref().to_string() }
    }
}

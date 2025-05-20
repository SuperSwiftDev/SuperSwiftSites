use std::collections::HashMap;

use crate::html::Html;
use crate::html::Element;

// ————————————————————————————————————————————————————————————————————————————
// PRETTY PRINTER
// ————————————————————————————————————————————————————————————————————————————

#[derive(Debug, Clone)]
pub struct PrettyPrinter {}

#[derive(Debug, Clone)]
pub struct Environment {
    indent: usize,
    format_type: FormatType,
}

impl Environment {
    pub fn scope(&self, tag: &str) -> Environment {
        let format_type = match self.format_type {
            FormatType::Block if crate::html::is_inline_tag(tag) => FormatType::Inline,
            _ => self.format_type
        };
        let auto_indent = match tag {
            "html" => false,
            "head" => false,
            "body" => false,
            _ => format_type == FormatType::Block,
        };
        Environment {
            indent: {
                if auto_indent {
                    self.indent + 1
                } else {
                    self.indent
                }
            },
            format_type: format_type,
        }
    }
    pub fn indent(self) -> Environment {
        Environment { indent: self.indent + 1, format_type: self.format_type }
    }
    pub fn inline(self) -> Environment {
        Environment {
            indent: self.indent,
            format_type: FormatType::Inline,
        }
    }
    fn indent_spacing_string(&self) -> String {
        indent_spacing_string(self.indent)
    }
    fn is_in_inline_mode(&self) -> bool {
        self.format_type == FormatType::Inline
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatType { Inline, Block }

impl Default for FormatType {
    fn default() -> Self {
        FormatType::Block
    }
}


impl Default for Environment {
    fn default() -> Self {
        Environment {
            indent: 0,
            format_type: FormatType::default()
        }
    }
}

// ————————————————————————————————————————————————————————————————————————————
// IMPLEMENTATION
// ————————————————————————————————————————————————————————————————————————————

impl Html {
    pub fn pretty_html_string(&self) -> String {
        let string = self.html_string(&Default::default());
        let pretty = crate::pretty_html::prettify_html(&string).unwrap_or_else(|error| {
            println!("PRETTY-HTML: {error}");
            string
        });
        pretty
    }
    pub fn html_string(&self, environment: &Environment) -> String {
        match self {
            Self::Element(element) => element.html_string(environment),
            Self::Fragment(nodes) => format_fragment(&nodes, environment),
            Self::Text(text) => text.to_owned(),
        }
    }
}

impl Element {
    pub fn html_string(&self, environment: &Environment) -> String {
        let environment = environment.scope(&self.tag);
        let level = environment.indent_spacing_string();
        let attributes = format_attributes(&self.attrs);
        if crate::html::is_void_tag(&self.tag) && self.children.len() == 0 {
            format!(
                "<{tag} {attributes} />",
                tag=self.tag,
            )
        } else {
            let children = format_fragment(&self.children, &environment);
            let contents = {
                children
            };
            format!(
                "<{tag} {attributes}>{contents}</{tag}>",
                tag=self.tag,
            )
        }
    }
}

fn format_fragment(nodes: &[Html], environment: &Environment) -> String {
    let xs = nodes
        .iter()
        .map(|child| {
            let environment = environment.clone();
            child.html_string(&environment)
        })
        .collect::<Vec<_>>();
    if xs.is_empty() {
        String::new()
    } else {
        xs.join("")
    }
}

fn format_attributes(attributes: &HashMap<String, String>) -> String {
    let mut attributes = attributes
        .iter()
        .map(|(key, value)| {
            // println!("{key:?}: {value:?}");
            // if value.is_empty() {
            //     return format!("{}", key);
            // }
            format!("{key}={value:?}")
        })
        .collect::<Vec<_>>();
    if attributes.is_empty() {
        String::new()
    } else {
        format!(" {}", attributes.join(" "))
    }
}

fn indent_spacing_string(level: usize) -> String {
    if level == 0 {
        String::from("")
    } else {
        std::iter::repeat(" ").take(level * 2).collect::<String>()
    }
}

// ————————————————————————————————————————————————————————————————————————————
// INTERNAL
// ————————————————————————————————————————————————————————————————————————————

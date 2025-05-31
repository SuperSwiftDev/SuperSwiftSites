use html5ever::tendril::TendrilSink;
use html5ever::tree_builder::{TreeSink, NodeOrText, ElementFlags, QuirksMode};
use html5ever::{parse_fragment, Attribute, QualName};
use html5ever::{ns, namespace_url}; // required for ns! macro
use tendril::Tendril;
use tendril::fmt::UTF8;

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

// === Your AST ===

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

// === Internal Tree Nodes ===

#[derive(Debug)]
enum NodeKind {
    Element(Element),
    Text(String),
    Comment(String),
    Fragment,
}

#[derive(Debug)]
struct Node {
    kind: NodeKind,
    children: Vec<usize>,
    parent: Option<usize>,
}

// === Interning Support ===

use html5ever::{LocalName, Namespace};

static LOCAL_NAME_CACHE: Lazy<Mutex<HashMap<String, &'static LocalName>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn get_local(name: &str) -> &'static LocalName {
    let mut cache = LOCAL_NAME_CACHE.lock().unwrap();
    if let Some(existing) = cache.get(name) {
        *existing
    } else {
        let interned: &'static LocalName = Box::leak(Box::new(LocalName::from(name)));
        cache.insert(name.to_string(), interned);
        interned
    }
}

static NAMESPACE_HTML: Lazy<&'static Namespace> = Lazy::new(|| {
    Box::leak(Box::new(ns!(html)))
});

static NAMESPACE_EMPTY: Lazy<&'static Namespace> = Lazy::new(|| {
    Box::leak(Box::new(ns!()))
});

// === Sink ===

struct FragmentSink {
    nodes: Vec<Node>,
    root_id: usize,
}

impl FragmentSink {
    fn new() -> Self {
        let root = Node {
            kind: NodeKind::Fragment,
            children: vec![],
            parent: None,
        };
        Self {
            nodes: vec![root],
            root_id: 0,
        }
    }

    fn to_html(&self, node_id: usize) -> Html {
        let node = &self.nodes[node_id];
        match &node.kind {
            NodeKind::Fragment => {
                Html::Fragment(node.children.iter().map(|&id| self.to_html(id)).collect())
            }
            NodeKind::Text(s) => Html::Text(s.clone()),
            NodeKind::Element(e) => {
                let mut el = e.clone();
                el.children = node.children.iter().map(|&id| self.to_html(id)).collect();
                Html::Element(el)
            }
            NodeKind::Comment(_) => Html::Fragment(vec![]), // ← skip it safely
        }
    }
}

impl TreeSink for FragmentSink {
    type Output = Html;
    type Handle = usize;

    fn finish(self) -> Self::Output {
        self.to_html(self.root_id)
    }

    fn parse_error(&mut self, _msg: std::borrow::Cow<'static, str>) {}

    fn get_document(&mut self) -> Self::Handle {
        self.root_id
    }

    fn get_template_contents(&mut self, _: &Self::Handle) -> Self::Handle {
        self.root_id
    }

    fn set_quirks_mode(&mut self, _: QuirksMode) {}

    fn same_node(&self, x: &Self::Handle, y: &Self::Handle) -> bool {
        x == y
    }

    fn elem_name(&self, target: &Self::Handle) -> html5ever::ExpandedName {
        use html5ever::ExpandedName;

        if let NodeKind::Element(el) = &self.nodes[*target].kind {
            ExpandedName {
                ns: *NAMESPACE_HTML,
                local: get_local(&el.tag),
            }
        } else {
            ExpandedName {
                ns: *NAMESPACE_EMPTY,
                local: get_local(""),
            }
        }
    }

    fn create_element(
        &mut self,
        name: QualName,
        attrs: Vec<Attribute>,
        _flags: ElementFlags,
    ) -> Self::Handle {
        let tag = name.local.to_string();
        let mut attr_map = HashMap::new();
        for attr in attrs {
            attr_map.insert(attr.name.local.to_string(), attr.value.to_string());
        }

        let el = Element {
            tag,
            attrs: attr_map,
            children: vec![],
        };

        let node = Node {
            kind: NodeKind::Element(el),
            children: vec![],
            parent: None,
        };

        self.nodes.push(node);
        self.nodes.len() - 1
    }

    fn create_comment(&mut self, _text: Tendril<UTF8>) -> Self::Handle {
        // Safe dummy node — never rendered
        let node = Node {
            kind: NodeKind::Comment(String::new()),
            children: vec![],
            parent: None,
        };

        self.nodes.push(node);
        self.nodes.len() - 1
    }

    fn create_pi(&mut self, _target: Tendril<UTF8>, _data: Tendril<UTF8>) -> Self::Handle {
        self.root_id
    }

    fn append(&mut self, parent: &Self::Handle, child: NodeOrText<Self::Handle>) {
        match child {
            NodeOrText::AppendNode(id) => {
                self.nodes[id].parent = Some(*parent);
                self.nodes[*parent].children.push(id);
            }
            NodeOrText::AppendText(text) => {
                // if text.chars().all(|c| c.is_whitespace()) {
                //     return; // or keep if you want to preserve even all-whitespace nodes
                // }
                let node = Node {
                    kind: NodeKind::Text(text.to_string()),
                    children: vec![],
                    parent: Some(*parent),
                };
                self.nodes.push(node);
                let id = self.nodes.len() - 1;
                self.nodes[*parent].children.push(id);
            }
        }
    }

    fn append_before_sibling(&mut self, _: &Self::Handle, _: NodeOrText<Self::Handle>) {}
    fn append_based_on_parent_node(&mut self, _: &Self::Handle, _: &Self::Handle, _: NodeOrText<Self::Handle>) {}
    fn append_doctype_to_document(&mut self, _: Tendril<UTF8>, _: Tendril<UTF8>, _: Tendril<UTF8>) {}
    fn add_attrs_if_missing(&mut self, _: &Self::Handle, _: Vec<Attribute>) {}
    fn remove_from_parent(&mut self, _: &Self::Handle) {}
    fn reparent_children(&mut self, _: &Self::Handle, _: &Self::Handle) {}
    fn mark_script_already_started(&mut self, _: &Self::Handle) {}
}

// === Public API ===

/// Parses a raw HTML fragment into your custom `Html` structure.
/// `context` is the tag name for the element under which parsing occurs (e.g. "div", "span", etc.)
pub fn parse_html_fragment(input: &str, context: &str) -> Html {
    // let context = QualName::new(None, *NAMESPACE_HTML, get_local(context).clone());
    let context = QualName::new(None, (*NAMESPACE_HTML).clone(), get_local(context).clone());

    let sink = FragmentSink::new();

    let root = parse_fragment(sink, Default::default(), context, vec![])
        .from_utf8()
        .read_from(&mut input.as_bytes())
        .unwrap();
    match root {
        Html::Element(element) if element.tag == "html" => Html::Fragment(element.children),
        Html::Fragment(nodes) if nodes.len() == 1 => {
            match nodes[0].clone() {
                Html::Element(element) if element.tag == "html" => Html::Fragment(element.children),
                _ => Html::Fragment(nodes),
            }
        }
        _ => root,
    }
}

pub fn parse_html_document(input: &str) -> Html {
    let sink = FragmentSink::new();

    html5ever::parse_document(sink, Default::default())
        .from_utf8()
        .read_from(&mut input.as_bytes())
        .unwrap()
}


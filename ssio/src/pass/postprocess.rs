use std::path::PathBuf;
use crate::html::Html;
use crate::html::Element;
use crate::dependency_tracking::resolve_virtual_paths::PathResolver;
use crate::dependency_tracking::resolve_virtual_paths::VirtualPathContext;
use crate::dependency_tracking::resolve_virtual_paths::resolve_virtual_paths;
// pub struct postprocess

#[derive(Debug, Clone)]
pub struct PostprocessEnvironment {
    /// The source file path this HTML node came from
    pub origin_file_path: PathBuf,
    /// The file path where the output will be written
    pub output_file_path: PathBuf,
    /// The virtual link resolver
    pub resolver: PathResolver,
}

impl PostprocessEnvironment {
    fn virtual_path_context(&self) -> VirtualPathContext {
        VirtualPathContext {
            origin_file_path: &self.origin_file_path,
            output_file_path: &self.output_file_path,
            resolver: &self.resolver,
        }
    }
}

impl Html {
    pub fn postprocess(self, env: &PostprocessEnvironment) -> Self {
        match self {
            Self::Element(element) => Self::Element(element.postprocess(env)),
            Self::Text(text) => Self::Text(text),
            Self::Fragment(nodes) => Self::Fragment(postprocess_fragment(nodes, env)),
        }
    }
}

impl Element {
    pub fn postprocess(self, env: &PostprocessEnvironment) -> Self {
        let Element { tag, mut attrs, children } = self;
        resolve_virtual_paths(&tag, &mut attrs, &env.virtual_path_context());
        let children = postprocess_fragment(children, env);
        Element { tag, attrs, children }
    }
}

fn postprocess_fragment(nodes: Vec<Html>, env: &PostprocessEnvironment) -> Vec<Html> {
    nodes
        .into_iter()
        .map(|node| node.postprocess(env))
        .collect()
}

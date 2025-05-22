use std::path::Path;

use crate::html::Html;
use crate::html::Element;
use crate::html::ParserMode;

use super::system::Scope;
use super::system::State;

pub fn load_html_file(
    file_path: impl AsRef<Path>,
    parser_mode: ParserMode,
    project_root: impl AsRef<Path>,
) -> Result<State<Html>, Box<dyn std::error::Error>> {
    let file_path = path_clean::clean(file_path.as_ref().to_path_buf());
    let source = std::fs::read_to_string(&file_path)?;
    let source_tree = Html::parse(&source, parser_mode);
    let scope = Scope {
        source_path: file_path,
        project_root: path_clean::clean(project_root.as_ref()),
    };
    Ok(process_html_tree(source_tree, &scope))
}

fn process_html_tree(html: Html, scope: &Scope) -> State<Html> {
    html.preprocess(scope)
}


use std::{collections::{HashMap, HashSet}, path::PathBuf};

// use pretty_tree::PrettyTreePrinter;

use crate::{html::ParserMode, process::{process_html_file, NavLinkRoute, OutputContext}};

#[derive(Debug, Clone)]
pub struct Compiler {
    pub template_path: Option<PathBuf>,
    pub input_paths: Vec<PathBuf>,
    pub output_dir: PathBuf,
    pub pretty_print: bool,
}

impl Compiler {
    pub fn run(&self) {
        std::fs::create_dir_all(&self.output_dir).unwrap();
        let template = self.template_path
            .as_ref()
            .map(|x| {
                process_html_file(x, crate::html::ParserMode::Document).unwrap()
            })
            .inspect(|out| {
                // out.value.print_pretty_tree();
            });
        let page_contents = self.input_paths
            .clone()
            .into_iter()
            .map(|file_path| {
                let source_io = process_html_file(&file_path, ParserMode::fragment("div")).unwrap();
                let baked_io = template
                    .clone()
                    .map(|template| {
                        crate::template::bake_template_content(template, source_io.clone(), true)
                    })
                    .unwrap_or_else(|| source_io.clone());
                // baked_io.value.print_pretty_tree();
                (file_path, baked_io)
            })
            .collect::<Vec<_>>();
        let env = page_contents
            .iter()
            .map(|(_, x)| x.context.clone())
            .fold(OutputContext::default(), |acc, x| { acc.merge(x) });
        let routes = env.routes
            .clone()
            .into_iter()
            .map(|x| (x.full_file_path(), x))
            .collect::<HashMap<_, _>>();
        let mut compiled_pages = HashSet::<PathBuf>::default();
        let route_pages = page_contents
            .into_iter()
            .filter_map(|(src_path, output)| {
                if let Some(route) = routes.get(&src_path) {
                    return Some((route.clone(), output))
                }
                None
            })
            .map(|(route, page)| {
                let output_target = self.output_dir.join(&route.external);
                (route, page, output_target)
            })
            .filter(|(route, contents, output_target)| {
                let keep = !compiled_pages.contains(output_target);
                if keep {
                    compiled_pages.insert(output_target.clone());
                }
                keep
            })
            .collect::<Vec<_>>();
        for (route, page, out_path) in route_pages {
            let page_str = if self.pretty_print {
                page.value.pretty_html_string()
            } else {
                let doctype = "<!DOCTYPE html>";
                format!(
                    "{doctype}{}",
                    page.value.html_string(&Default::default()),
                )
            };
            let should_write = std::fs::read_to_string(&out_path)
                .map(|current| {
                    current !=  page_str
                })
                .unwrap_or(true);
            if should_write {
                std::fs::write(&out_path, page_str).unwrap();
            }
        }
    }
}


impl OutputContext {
    pub fn notable_files(&self) -> HashSet<PathBuf> {
        let xs = self.routes
            .clone()
            .into_iter()
            .map(|x| {
                x.full_file_path()
            })
            .collect::<HashSet<_>>();
        xs
    }
}

impl NavLinkRoute {
    fn full_file_path(&self) -> PathBuf {
        let base = self.origin.parent().unwrap();
        let full = base.join(&self.internal);
        let full = path_clean::clean(&full);
        full
    }
    fn matches(&self, target_path: impl AsRef<std::path::Path>) -> bool {
        target_path.as_ref() == self.full_file_path().as_path()
    }
}

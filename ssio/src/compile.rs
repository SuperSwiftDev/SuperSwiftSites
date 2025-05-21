use std::{collections::{HashMap, HashSet}, path::PathBuf};

// use pretty_tree::PrettyTreePrinter;

use pretty_tree::PrettyTreePrinter;

use crate::{html::ParserMode, pass::resolve_virtual_paths::{PathResolver, VirtualPathContext}, process::{process_html_file, Dependency, OutputContext, SiteLink}};

#[derive(Debug, Clone)]
pub struct Compiler {
    pub project_root: PathBuf,
    pub template_path: Option<PathBuf>,
    pub input_paths: Vec<InputRule>,
    pub output_dir: PathBuf,
    pub pretty_print: bool,
}

/// Input file with optional rewrite rule
#[derive(Debug, Clone)]
pub struct InputRule {
    /// Input file path
    pub source: PathBuf,
    /// Desired output path
    pub target: Option<PathBuf>,
}

impl Compiler {
    pub fn run(&self) {
        std::fs::create_dir_all(&self.output_dir).unwrap();
        let template_path = self.template_path.as_ref();
        let template = template_path
            .map(|path| {
                match process_html_file(path, crate::html::ParserMode::Document, &self.project_root) {
                    Ok(x) => x,
                    Err(error) => {
                        eprintln!("Failed to read file: {path:?}");
                        panic!("{error}")
                    }
                }
            })
            .inspect(|out| {
                // out.value.print_pretty_tree();
            });
        let page_contents = self.input_paths
            .clone()
            .into_iter()
            .map(|rule| {
                let source_io = process_html_file(&rule.source, ParserMode::fragment("div"), &self.project_root).unwrap();
                let baked_io = template
                    .clone()
                    .map(|template| {
                        crate::template::bake_template_content(template, source_io.clone(), true)
                    })
                    .inspect(|baked| {
                        // baked.value.print_pretty_tree();
                    })
                    .map(|template| {
                        if let Some(template_path) = template_path {
                            template
                                // .map(|template| {
                                //     let path_rewrite = crate::pass::path_rewrite::PathRewrite {
                                //         from_original: path_clean::clean(template_path),
                                //         to_target: path_clean::clean(file_path.clone()),
                                //     };
                                //     template.apply_path_rewrite(&path_rewrite)
                                // })
                        } else {
                            template
                        }
                    })
                    .unwrap_or_else(|| source_io.clone());
                // baked_io.value.print_pretty_tree();
                (rule.source, baked_io, rule.target)
            })
            .map(|(src_path, page, out_path)| {
                let out_path = out_path
                    // .inspect(|out| {
                    //     println!(">> {src_path:?} => {out:?}")
                    // })
                    .map(|out| {
                        self.output_dir.join(out)
                    })
                    .unwrap_or_else(|| {
                        let out = src_path.strip_prefix(&self.project_root).unwrap();
                        let out = out.to_path_buf();
                        // println!("[<{:?}>] {src_path:?} {out:?}", self.project_root);
                        self.output_dir.join(out)
                    });
                (src_path, page, out_path)
            })
            .collect::<Vec<_>>();
        let env = page_contents
            .iter()
            .map(|(_, x, _)| x.context.clone())
            .fold(OutputContext::default(), |acc, x| { acc.merge(x) });
        // println!("{env:#?}");
        let dependencies = env.dependencies
            .clone()
            .into_iter()
            .filter(|x| !x.internal)
            .filter(|x| {
                let full_resolved_path = x.resolved_source_file_path();
                full_resolved_path.exists()
            })
            .collect::<Vec<_>>();
        let site_links = env.site_link
            .clone()
            .into_iter()
            // .map(|x| (x.full_file_path(), x))
            .flat_map(|x| {
                vec![
                    // (x.full_file_path(), x.clone()),
                    // (x.target.clone(), x.clone()),
                    (x.normalized_target(), x),
                ]
            })
            .collect::<HashMap<_, _>>();
        let mut compiled_pages = HashSet::<PathBuf>::default();
        let route_pages = page_contents
            .into_iter()
            // .filter_map(|(src_path, output, _)| {
            //     if let Some(route) = site_links.get(&src_path) {
            //         return Some((route.clone(), output))
            //     }
            //     println!("NOPE: {src_path:?}");
            //     None
            // })
            // .map(|(source_path, page, out_target)| {
            //     let external = route.public
            //         .as_ref()
            //         .and_then(|x| {
            //             Option::<&String>::None
            //         })
            //         .map(|public| {
            //             public
            //                 .strip_prefix("/")
            //                 .map(ToOwned::to_owned)
            //                 .unwrap_or(public.clone())
            //         })
            //         .unwrap_or_else(|| {
            //             let target = route.target.clone();
            //             let target = target.to_str().unwrap().to_string();
            //             target
            //         });
            //     let output_target = self.output_dir.join(&external);
            //     (route, page, output_target)
            // })
            // .filter(|(route, contents, output_target)| {
            //     let keep = !compiled_pages.contains(output_target);
            //     if keep {
            //         compiled_pages.insert(output_target.clone());
            //     }
            //     keep
            // })
            .collect::<Vec<_>>();
        let asset_inputs = env.dependencies
            .iter()
            .filter(|x| !x.internal)
            .map(|x| x.resolved_source_file_path())
            // .map(|x| x.resolved_target_file_path(&self.output_dir))
            .map(|x| path_clean::clean(x))
            .collect::<Vec<_>>();
        for dependency in dependencies {
            let full_resolved_path = dependency.resolved_source_file_path();
            let target_path = dependency.resolved_target_file_path(&self.output_dir);
            // println!("{dependency:?}: {:?} => {:?}", full_resolved_path, target_path);
            crate::symlink::create_relative_symlink(
                &full_resolved_path,
                &target_path
            ).unwrap();
        }
        // println!("asset_inputs: {asset_inputs:#?}");
        for (src_path, page, out_path) in route_pages {
            assert!(out_path != src_path);
            assert!(out_path.starts_with(&self.output_dir));
            let context = VirtualPathContext {
                output_file_path: out_path.clone(),
                origin_file_path: src_path.clone(),
                resolver: PathResolver {
                    source_input_rules: self.input_paths.clone(),
                    asset_input_rules: asset_inputs.clone(),
                    project_root: self.project_root.clone(),
                    output_dir: self.output_dir.clone(),
                },
            };
            // println!("{context:#?}");
            let page_html = page.value.resolve_virtual_paths(&context);
            let page_str = if self.pretty_print {
                page_html.pretty_html_string()
            } else {
                let doctype = "<!DOCTYPE html>";
                format!(
                    "{doctype}{}",
                    page_html.html_string(&Default::default()),
                )
            };
            let should_write = std::fs::read_to_string(&out_path)
                .map(|current| {
                    current !=  page_str
                })
                .unwrap_or(true);
            if should_write {
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent).unwrap();
                }
                std::fs::write(&out_path, page_str).unwrap();
            }
        }
    }
}


// impl OutputContext {
//     pub fn notable_files(&self) -> HashSet<PathBuf> {
//         let xs = self.routes
//             .clone()
//             .into_iter()
//             .map(|x| {
//                 x.full_file_path()
//             })
//             .collect::<HashSet<_>>();
//         xs
//     }
// }

impl SiteLink {
    // fn full_file_path(&self) -> PathBuf {
    //     let base = self.origin.parent().unwrap();
    //     let target = self.public
    //         .clone()
    //         .unwrap_or_else(|| {
    //             let target = self.target.clone();
    //             let target = target.to_str().unwrap().to_string();
    //             target
    //         });
    //     let full = base.join(target);
    //     let full = path_clean::clean(&full);
    //     full
    // }
    // fn matches(&self, target_path: impl AsRef<std::path::Path>) -> bool {
    //     target_path.as_ref() == self.full_file_path().as_path()
    // }
    fn normalized_target(&self) -> PathBuf {
        let origin_dir = self.origin.parent().unwrap();
        let normalized_target = origin_dir.join(&self.target);
        normalized_target
    }
}

impl Dependency {
    fn resolved_source_file_path(&self) -> PathBuf {
        let base = self.origin.parent().unwrap();
        let full = base.join(&self.target);
        let full = path_clean::clean(&full);
        full
    }
    fn resolved_target_file_path(&self, output_dir: impl AsRef<std::path::Path>) -> PathBuf {
        output_dir.as_ref().join(&self.target)
    }
}

use lightningcss::printer::PrinterOptions;
use lightningcss::stylesheet::{ParserOptions, StyleSheet};
use lightningcss::traits::ToCss;
use lightningcss::rules::CssRule;
use lightningcss::values::length::LengthValue;
use lightningcss::values::url::Url;
use lightningcss::visit_types;
use std::convert::Infallible;
use std::path::PathBuf;
use lightningcss::visitor::{Visit, VisitTypes, Visitor};

use crate::html_pass::postprocess::PostprocessEnvironment;
use crate::html_pass::system::Scope;
use crate::html_pass::system::Aggregator;
use crate::html_pass::system::Dependency;

pub fn pre_process(source_code: &str, scope: &Scope, aggregator: &mut Aggregator) -> String {
    let mut stylesheet = StyleSheet::parse(source_code, ParserOptions::default()).unwrap();
    
    let mut visitor = CssPreprocessVisitor {
        scope,
        aggregator,
    };
    
    stylesheet.visit(&mut visitor ).unwrap();
    
    let res: lightningcss::stylesheet::ToCssResult = stylesheet.to_css(PrinterOptions { minify: false, ..Default::default() }).unwrap();

    res.code
}

pub fn post_process(source_code: &str, env: &PostprocessEnvironment) -> String {
    let mut stylesheet = StyleSheet::parse(source_code, ParserOptions::default()).unwrap();
    
    let mut visitor = CssPostprocessVisitor {
        environment: env,
    };
    
    stylesheet.visit(&mut visitor ).unwrap();
    
    let res: lightningcss::stylesheet::ToCssResult = stylesheet.to_css(PrinterOptions { minify: false, ..Default::default() }).unwrap();

    res.code
}


struct CssPreprocessVisitor<'a> {
    scope: &'a Scope,
    aggregator: &'a mut Aggregator,
}

impl<'a, 'i> Visitor<'i> for CssPreprocessVisitor<'a> {
    type Error = Infallible;

    fn visit_types(&self) -> VisitTypes {
        visit_types!(URLS)
    }

    fn visit_url(&mut self, url: &mut Url<'i>) -> Result<(), Self::Error> {
        let url_str = url.url.to_string();
        if crate::path_utils::is_external_url(&url_str) {
            return Ok(())
        }
        let virtual_src = crate::path_utils::normalize_virtual_path(
            &url_str,
            &self.scope.source_path,
            &self.scope.project_root,
        );
        let source = path_clean::clean(self.scope.source_path.clone());
        let target = path_clean::clean(PathBuf::from(&url_str));
        // println!("normalize_virtual_path [{source:?}]: {target:?} -> {virtual_src:?}");
        self.aggregator.static_dependencies.insert(
            Dependency {
                origin: source,
                target,
                is_internal: None
            }
        );
        url.url = virtual_src.into();
        Ok(())
    }
}

struct CssPostprocessVisitor<'a> {
    environment: &'a PostprocessEnvironment,
}

impl<'a, 'i> Visitor<'i> for CssPostprocessVisitor<'a> {
    type Error = Infallible;

    fn visit_types(&self) -> VisitTypes {
        visit_types!(URLS)
    }

    fn visit_url(&mut self, url: &mut Url<'i>) -> Result<(), Self::Error> {
        // println!("resolve_virtual_path: {:?}", url.url);
        let href = url.url.to_string();
        let resolved = crate::dependency_tracking::resolve_virtual_paths::to_resolved_path(
            &href,
            &self.environment.origin_file_path,
            &self.environment.output_file_path,
            &self.environment.resolver
        );
        url.url = resolved.into();
        Ok(())
    }
}


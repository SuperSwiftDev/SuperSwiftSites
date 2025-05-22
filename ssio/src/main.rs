#![allow(unused)]
use clap::Parser;

#[macro_use] extern crate html5ever;
#[macro_use] extern crate markup5ever;

pub mod html;
pub mod html_parser;
pub mod html_string;
pub mod pretty_html;
pub mod cli;
pub mod compile;
pub mod process;
pub mod template;
pub mod manifest;
pub mod symlink;
pub mod pass;
pub mod path_utils;
pub mod dependency_tracking;

fn main() {
    cli::Cli::parse().execute();
}

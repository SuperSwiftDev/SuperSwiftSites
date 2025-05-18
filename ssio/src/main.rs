#![allow(unused)]
use clap::Parser;

pub mod html;
pub mod html_parser;
pub mod html_string;
pub mod pretty_html;
pub mod cli;
pub mod compile;
pub mod process;
pub mod template;

fn main() {
    cli::Cli::parse().execute();
}

use std::process::{Command, Stdio};
use std::io::Write;
use std::fmt;

#[derive(Debug)]
pub enum HtmlPrettifyError {
    TidyNotInstalled,
    TidyExecutionFailed(String),
    Utf8ConversionError(std::string::FromUtf8Error),
}

impl fmt::Display for HtmlPrettifyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HtmlPrettifyError::TidyNotInstalled => write!(f, "The 'tidy' CLI tool is not installed or not found in PATH."),
            HtmlPrettifyError::TidyExecutionFailed(msg) => write!(f, "Failed to prettify HTML: {}", msg),
            HtmlPrettifyError::Utf8ConversionError(e) => write!(f, "UTF-8 error: {}", e),
        }
    }
}

impl std::error::Error for HtmlPrettifyError {}

/// Prettifies HTML using the `tidy` CLI tool.
pub fn prettify_html(html: &str) -> Result<String, HtmlPrettifyError> {
    let mut child = Command::new("tidy")
        .args(&[
            "-quiet",          // suppress warnings
            "--show-warnings",
            "no",
            "-indent",         // pretty print
            "-wrap", "120",    // wrap lines at 120 chars
            "-as-html",        // treat input as HTML
            "-utf8",           // set output encoding
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null()) // ignore stderr unless debugging
        .spawn()
        .map_err(|e| {
            eprintln!("WARNING: {e}");
            HtmlPrettifyError::TidyNotInstalled
        })?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(html.as_bytes()).unwrap();
    }

    let output = child
        .wait_with_output()
        .map_err(|e| {
            eprintln!("WARNING: {e}");
            HtmlPrettifyError::TidyExecutionFailed(e.to_string())
        })?;

    // if !output.status.success() {
    //     eprintln!("WARNING: TIDY CLI FAILED");
    //     return Err(HtmlPrettifyError::TidyExecutionFailed(format!(
    //         "Exit code: {}",
    //         output.status
    //     )));
    // }
    let status = output.status.code().unwrap_or(-1);
    if status != 0 && status != 1 {
        eprintln!("WARNING: TIDY CLI FAILED");
        return Err(HtmlPrettifyError::TidyExecutionFailed(format!(
            "Exit code: {}",
            status
        )));
    }

    String::from_utf8(output.stdout).map_err(HtmlPrettifyError::Utf8ConversionError)
}

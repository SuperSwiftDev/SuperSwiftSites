use once_cell::sync::Lazy;
use std::collections::HashSet;


pub static REQUIRES_REGULAR_DEPENDENCY_TRACKING: Lazy<HashSet<(&'static str, &'static str)>> = Lazy::new(|| {
    HashSet::from([
        ("a", "href"),
        ("area", "href"),
        ("link", "href"),
        ("img", "src"),
        ("video", "src"),
        ("video", "poster"),
        ("source", "src"),
        ("script", "src"),
        ("iframe", "src"),
        ("audio", "src"),
        ("track", "src"),
        ("embed", "src"),
        ("object", "data"),
        ("form", "action"),
        ("input", "formaction"),
        ("button", "formaction"),
        ("use", "href"),
        ("use", "xlink:href"),
        ("image", "href"),
        ("image", "xlink:href"),
    ])
});

pub static REQUIRES_SRC_SET_DEPENDENCY_TRACKING: Lazy<HashSet<(&'static str, &'static str)>> = Lazy::new(|| {
    HashSet::from([
        ("img", "srcset"),
        ("source", "srcset"),
    ])
});

pub static REQUIRES_DYNAMIC_SITE_LINK_DEPENDENCY_TRACKING: Lazy<HashSet<(&'static str, &'static str)>> = Lazy::new(|| {
    HashSet::from([
        ("a", "href"),
    ])
});

pub static TAG_MAY_REQUIRE_DEPENDENCY_TRACKING: Lazy<HashSet<&'static str>> = Lazy::new(|| { tags_only() });

fn tags_only() -> HashSet<&'static str> {
    let xs = REQUIRES_REGULAR_DEPENDENCY_TRACKING
        .iter()
        .chain(REQUIRES_SRC_SET_DEPENDENCY_TRACKING.iter())
        .map(|(x, _)| *x);
    let result: HashSet<&'static str> = HashSet::from_iter(xs);
    result
}

#[derive(Debug, Clone, PartialEq)]
pub struct SrcsetCandidate {
    pub url: String,
    pub descriptor: Option<String>,
}

impl SrcsetCandidate {
    pub fn parse_srcset(input: &str) -> Vec<Self> {
        let mut input = input.trim();
        let mut output = Vec::new();
    
        while !input.is_empty() {
            // 1. Skip leading whitespace
            input = input.trim_start();
    
            // 2. Extract URL
            let mut url_end = 0;
            let mut in_url = false;
            for (i, c) in input.char_indices() {
                if c == ',' || c.is_whitespace() {
                    break;
                }
                in_url = true;
                url_end = i + c.len_utf8();
            }
    
            if !in_url {
                break;
            }
    
            let url = &input[..url_end];
            input = &input[url_end..];
    
            // 3. Skip whitespace after URL
            input = input.trim_start();
    
            // 4. Parse descriptor (if any)
            let mut descriptor = None;
            if !input.is_empty() && !input.starts_with(',') {
                let mut desc_end = 0;
                for (i, c) in input.char_indices() {
                    if c == ',' {
                        break;
                    }
                    desc_end = i + c.len_utf8();
                }
    
                if desc_end > 0 {
                    let desc = input[..desc_end].trim();
                    if !desc.is_empty() {
                        descriptor = Some(desc.to_string());
                    }
                    input = &input[desc_end..];
                }
            }
    
            output.push(SrcsetCandidate {
                url: url.to_string(),
                descriptor,
            });
    
            // 5. Skip over comma
            if let Some(pos) = input.find(',') {
                input = &input[pos + 1..];
            } else {
                break;
            }
        }
    
        output
    }
    pub fn format_srcset(candidates: &[SrcsetCandidate]) -> String {
        candidates
            .iter()
            .map(|c| {
                if let Some(desc) = &c.descriptor {
                    format!("{} {}", c.url, desc)
                } else {
                    c.url.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}



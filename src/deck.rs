use crate::rundown::Rundown;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Deck {
    pub slides: Vec<Slide>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slide {
    pub section: Option<String>,
    pub cue: String,
    pub body: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Bullet(Vec<Span>),
    Para(Vec<Span>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub style: SpanStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanStyle {
    Plain,
    Bold,
    Italic,
    Code,
}

/// Total: any text yields a deck. Lines before the first `##` belong to no slide.
pub fn parse(markdown: &str) -> Deck {
    Rundown::import(markdown, "").deck()
}

/// The body of one card: bullets, paragraphs, and deeper headings as bold lines.
pub fn parse_body(text: &str) -> Vec<Block> {
    let mut body = Vec::new();
    let mut para = String::new();
    let flush = |body: &mut Vec<Block>, para: &mut String| {
        if !para.is_empty() {
            body.push(Block::Para(spans(para)));
            para.clear();
        }
    };
    for line in text.lines() {
        let line = line.trim();
        if let Some((_, text)) = heading(line) {
            flush(&mut body, &mut para);
            body.push(Block::Para(vec![Span {
                text: text.to_string(),
                style: SpanStyle::Bold,
            }]));
        } else if line.is_empty() {
            flush(&mut body, &mut para);
        } else if let Some(item) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            flush(&mut body, &mut para);
            body.push(Block::Bullet(spans(item.trim())));
        } else {
            if !para.is_empty() {
                para.push(' ');
            }
            para.push_str(line);
        }
    }
    flush(&mut body, &mut para);
    body
}

pub(crate) fn heading(line: &str) -> Option<(usize, &str)> {
    let text = line.trim_start_matches('#');
    let level = line.len() - text.len();
    if level == 0 || !(text.is_empty() || text.starts_with(' ')) {
        return None;
    }
    Some((level, text.trim()))
}

fn spans(text: &str) -> Vec<Span> {
    let mut out = Vec::new();
    let mut plain = String::new();
    let mut rest = text;
    while let Some(i) = rest.find(['*', '`']) {
        plain.push_str(&rest[..i]);
        let (marker, style) = if rest[i..].starts_with("**") {
            ("**", SpanStyle::Bold)
        } else if rest[i..].starts_with('*') {
            ("*", SpanStyle::Italic)
        } else {
            ("`", SpanStyle::Code)
        };
        let after = &rest[i + marker.len()..];
        match after.find(marker) {
            Some(j)
                if !after[..j].starts_with(char::is_whitespace)
                    && !after[..j].ends_with(char::is_whitespace)
                    && j > 0 =>
            {
                if !plain.is_empty() {
                    out.push(Span {
                        text: std::mem::take(&mut plain),
                        style: SpanStyle::Plain,
                    });
                }
                out.push(Span {
                    text: after[..j].to_string(),
                    style,
                });
                rest = &after[j + marker.len()..];
            }
            _ => {
                plain.push_str(marker);
                rest = after;
            }
        }
    }
    plain.push_str(rest);
    if !plain.is_empty() {
        out.push(Span {
            text: plain,
            style: SpanStyle::Plain,
        });
    }
    out
}

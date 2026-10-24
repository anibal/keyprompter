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
    let mut deck = Deck::default();
    let mut section = None;
    let mut open: Option<Slide> = None;
    let mut para = String::new();

    fn flush_para(open: &mut Option<Slide>, para: &mut String) {
        if let Some(slide) = open
            && !para.is_empty()
        {
            slide.body.push(Block::Para(spans(para)));
        }
        para.clear();
    }

    for line in markdown.lines() {
        let line = line.trim();
        if let Some((level, text)) = heading(line) {
            flush_para(&mut open, &mut para);
            match level {
                1 | 2 => {
                    deck.slides.extend(open.take());
                    if level == 1 {
                        section = Some(text.to_string());
                    } else {
                        open = Some(Slide {
                            section: section.clone(),
                            cue: text.to_string(),
                            body: Vec::new(),
                        });
                    }
                }
                _ => {
                    if let Some(slide) = &mut open {
                        slide.body.push(Block::Para(vec![Span {
                            text: text.to_string(),
                            style: SpanStyle::Bold,
                        }]));
                    }
                }
            }
        } else if line.is_empty() {
            flush_para(&mut open, &mut para);
        } else if open.is_some() {
            if let Some(item) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
                flush_para(&mut open, &mut para);
                if let Some(slide) = &mut open {
                    slide.body.push(Block::Bullet(spans(item.trim())));
                }
            } else {
                if !para.is_empty() {
                    para.push(' ');
                }
                para.push_str(line);
            }
        }
    }
    flush_para(&mut open, &mut para);
    deck.slides.extend(open);
    deck
}

fn heading(line: &str) -> Option<(usize, &str)> {
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

use crate::tokenizer::{Emphasis, LinkKind, MarkdownTag};


#[derive(Debug)]
pub struct Document {
    title: String,
    body: Vec<String>,
}

impl Document {
    pub fn new(title: &str) -> Self {
        Document {
            title: format!("<title> Maple - {} </title>", title),
            body: Vec::new(),
        }

    }

    pub fn title(&self) -> String {
        self.title.clone()
    }

    pub fn output(&self) -> String {
        let mut body = String::new();
        body.push_str(&self.title);

        for content in self.body.clone() {
            body.push_str(&content);
        }

        body
    }

    // appends to the body tag
    pub fn append(&mut self, content: String) {
        self.body.push(content);
    }

    pub fn append_void(&mut self, tag: &str) {
        self.body.push(format!("<{} />", tag));
    }

    pub fn append_wrapped_with_attr(&mut self, tag: &str, attr: &str, content: &str) {
        self.body.push(format!("<{} {}>{}</{}>", tag, attr, content, tag));
    }

    pub fn append_wrapped(&mut self, tag: &str, content: &str) {
        self.body.push(format!("<{}>{}</{}>", tag, content, tag));
    }

    pub fn append_newline(&mut self) {
        self.body.push(String::from("<br>\n"));
    }

    pub fn append_code(&mut self, content: &str) {
        self.append_wrapped("code", &content);
    }

    pub fn append_text(&mut self, emphasis: Option<Emphasis>, content: &str) {
        if emphasis.is_none() {
            self.append(content.to_string());
        } else {
            match emphasis.unwrap() {
                Emphasis::Bold => {
                    self.append_wrapped("b", content);
                },
                Emphasis::Italic => {
                    self.append_wrapped("i", content);
                },
                Emphasis::Strikethrough => {
                    self.append_wrapped("s", content);
                },
            }
        }
    }

    pub fn add_markdown(&mut self, markdown: MarkdownTag) {
        match markdown {
            MarkdownTag::Header(level, content ) => {
                let header_count: usize = level.into();
                self.append_wrapped(&format!("h{}", header_count), &content);
            },
            MarkdownTag::LineSeparator => {
                self.append_void("hr");
            }, 
            MarkdownTag::Checkbox(state, content) => {
                let checked = if state { "checked" } else { "" };

                self.append_wrapped_with_attr("input", &format!("type=\"checkbox\" disabled {}", checked), "");

                if state {
                    self.append_text(Some(Emphasis::Strikethrough), &content);
                } else {
                    self.append_text(None, &content);
                }

                self.append_newline();
            },
            MarkdownTag::BulletPoint(content) => {
                self.append_wrapped("li", &content);
            },
            MarkdownTag::Blockquote(content) => {
                self.append_wrapped("blockquote", &content);
            },
            MarkdownTag::CodeBlock(content) => {
                let content = content.replace("\n", "<br>");
                self.append_wrapped("pre", &content);
            },
            MarkdownTag::Link(kind, display, link) => {
                match kind {
                    LinkKind::Image => {
                        self.append_newline();
                        self.append_wrapped_with_attr("img", &format!("src=\"{}\" alt=\"{}\"", link, display), "");
                    },
                    LinkKind::Hyperlink => {
                        self.append_wrapped_with_attr("a", &format!("href=\"{}\" target=\"_blank\" ", link), &display);
                    }
                }
            }
        }
    }
}

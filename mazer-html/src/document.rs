use mazer_parser::AST;



enum FontKind {
    Bold,
    Italic,
    Underline,
    Strikethrough,
}

pub struct Document {
    head: String,
    body: Vec<String>,
    nodes: Vec<AST>,
}

impl Document {

    pub fn new(nodes: Vec<AST>) -> Self {

        let head = "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"UTF-8\"><link rel=\"icon\" href=\"data:image/svg+xml,<svg xmlns=%22http://www.w3.org/2000/svg%22 viewBox=%220 0 32 32%22><text y=%2232%22 font-size=%2232%22>🍁</text></svg>\"><script src=\"https://cdn.jsdelivr.net/npm/@arborium/arborium/dist/arborium.iife.js\" data-theme=\"github-light\" data-selector=\"pre code\"></script></head>";

        Document {
            head: String::from(head),
            body: Vec::new(),
            nodes,
        }
    }

    pub fn build(&mut self) {
        self.append("<body>");
        for node in &self.nodes.clone() {
            self.append_node(node.clone());
        }
        self.append("</body>");
    }

    pub fn output(&self) -> String {
        let mut html = String::with_capacity(1024);
        html.push_str(&self.head);
        for content in &self.body {
            html.push_str(content);
        }
        html.push_str("</html>");
        html
    }

    fn append(&mut self, content: &str) {
        self.body.push(String::from(content));
    }

    fn append_node(&mut self, node: AST) {
        match node {
            AST::Header { level, text } => {
                self.append_header(level, text);
            },
            AST::UnorderedList { items } => {
                self.append_unordered_list(items);
            },
            AST::CheckboxUnchecked { text } => {
                self.append_checkbox(text, false);
            },
            AST::CheckboxChecked { text } => {
                self.append_checkbox(text, true);
            },
            AST::BlockQuote { content } => {
                self.append_blockquote(content);
            },
            AST::Spoiler { content } => {
                self.append_spoiler(content);
            },
            AST::Link { text, url } => {
                self.append_link(text, url);
            },
            AST::CodeBlock { code, language } => {
                self.append_codeblock(code, language);
            },
            AST::InlineCode { code } => {
                self.append_inline_code(code);
            },
            AST::Bold { text } => {
                self.append_text(text, FontKind::Bold);
            },
            AST::Italic { text } => {
                self.append_text(text, FontKind::Italic);
            },
            AST::Underline { text } => {
                self.append_text(text, FontKind::Underline);
            },
            AST::Strikethrough { text } => {
                self.append_text(text, FontKind::Strikethrough);
            },
            AST::PageSeparator => {
                self.append_page_separator();
            },
            AST::EvalBlock { code } => {
                dbg!(&code);
                self.append(&code);
            },
            AST::ShowBlock { code } => {
                dbg!(&code);
                self.append(&code);
            },
            AST::Text { content } => {
                self.append(&content);
            },
            AST::Paragraph { children } => {
                for c in children {
                    self.append_node(c);
                }
            }
        }
    }

    #[inline]
    fn append_header(&mut self, level: u8, text: String) {
        let level = level.clamp(1, 6);
        let header_html = format!("<h{level}>{}</h{level}>", text);
        self.append(&header_html);
    }

    #[inline]
    fn append_unordered_list(&mut self, items: Vec<String>) {
        let mut list_html = String::from("<ul>");
        for item in items {
            list_html.push_str(&format!("<li>{}</li>", item));
        }
        list_html.push_str("</ul>");
        self.append(&list_html);
    }

    #[inline]
    fn append_checkbox(&mut self, text: String, checked: bool) {
        let checkbox_html = if checked {
            format!("<input type=\"checkbox\" checked disabled> {}<br>", text)
        } else {
            format!("<input type=\"checkbox\" disabled> {}<br>", text)
        };
        self.append(&checkbox_html);
    }

    #[inline]
    fn append_blockquote(&mut self, content: String) {
        let blockquote_html = format!("<blockquote>{}</blockquote>", content);
        self.append(&blockquote_html);
    }

    #[inline]
    fn append_spoiler(&mut self, content: String) {
        let begin = "<span style=\"background:#2f3136;color:transparent;border-radius:3px;padding:0 4px;cursor:pointer\" onmouseover=\"this.style.color='#dcddde'\" onmouseout=\"this.style.color='transparent'\">";
        let end = "</span>";
        let spoiler_html = format!("{}{}{}", begin, content, end);
        self.append(&spoiler_html);
    }

    #[inline]
    fn append_link(&mut self, text: String, url: String) {
        // always open in new tab
        let link_html = format!("<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">{}</a>", url, text);
        self.append(&link_html);
    }

    #[inline]
    fn append_codeblock(&mut self, code: String, language: Option<String>) {
        let lang_html = format!("<pre> <code class=\"language-{}\">{}</code></pre>", language.unwrap_or_default(), code);
        self.append(&lang_html);
    }

    #[inline]
    fn append_inline_code(&mut self, code: String) {
        let inline_code_html = format!("<code>{}</code>", code);
        self.append(&inline_code_html);
    }

    #[inline]
    fn append_text(&mut self, text: String, kind: FontKind) {
        let formatted_text = match kind {
            FontKind::Bold => format!("<strong>{}</strong>", text),
            FontKind::Italic => format!("<em>{}</em>", text),
            FontKind::Underline => format!("<u>{}</u>", text),
            FontKind::Strikethrough => format!("<s>{}</s>", text),
        };
        self.append(&formatted_text);
    }

    #[inline]
    fn append_page_separator(&mut self) {
        self.append("<hr/>");
    }

}

use mazer_lisp::{ast::LispAST, parser::Parser};
use mazer_parser::MdAst;


#[derive(Debug, Clone)]
pub enum LispFragments {
    Eval(LispAST),
    Show(LispAST),
}

enum FontKind {
    Bold,
    Italic,
    Underline,
    Strikethrough,
}

pub struct Document {
    head: String,
    body: Vec<String>,
    nodes: Vec<MdAst>,
    frags: Vec<LispFragments>,
}

impl Document {

    pub fn new(nodes: Vec<MdAst>) -> Self {

        let head = "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"UTF-8\"><link rel=\"icon\" href=\"data:image/svg+xml,<svg xmlns=%22http://www.w3.org/2000/svg%22 viewBox=%220 0 32 32%22><text y=%2232%22 font-size=%2232%22>🍁</text></svg>\"><script src=\"https://cdn.jsdelivr.net/npm/@arborium/arborium/dist/arborium.iife.js\" data-theme=\"github-light\" data-selector=\"pre code\"></script></head>";

        Document {
            head: String::from(head),
            body: Vec::new(),
            nodes,
            frags: Vec::with_capacity(256),
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

    fn append_node(&mut self, node: MdAst) {
        match node {
            MdAst::Header { level, text } => {
                self.append_header(level, text);
            },
            MdAst::UnorderedList { items } => {
                self.append_unordered_list(items);
            },
            MdAst::CheckboxUnchecked { text } => {
                self.append_checkbox(text, false);
            },
            MdAst::CheckboxChecked { text } => {
                self.append_checkbox(text, true);
            },
            MdAst::BlockQuote { content } => {
                self.append_blockquote(content);
            },
            MdAst::Spoiler { content } => {
                self.append_spoiler(content);
            },
            MdAst::Link { text, url } => {
                self.append_link(text, url);
            },
            MdAst::CodeBlock { code, language } => {
                self.append_codeblock(code, language);
            },
            MdAst::InlineCode { code } => {
                self.append_inline_code(code);
            },
            MdAst::Bold { text } => {
                self.append_text(text, FontKind::Bold);
            },
            MdAst::Italic { text } => {
                self.append_text(text, FontKind::Italic);
            },
            MdAst::Underline { text } => {
                self.append_text(text, FontKind::Underline);
            },
            MdAst::Strikethrough { text } => {
                self.append_text(text, FontKind::Strikethrough);
            },
            MdAst::PageSeparator => {
                self.append_page_separator();
            },
            MdAst::EvalBlock { code } => {
                self.frags.push(LispFragments::Eval({
                    let p  = Parser::new(&code).parse().map_err(|e| e.to_string()).expect("Failed to parse lisp code");
                    p
                }));
                self.append(&code);
            },
            MdAst::ShowBlock { code } => {
                self.frags.push(LispFragments::Show({
                    let p  = Parser::new(&code).parse().map_err(|e| e.to_string()).expect("Failed to parse lisp code");
                    p
                }));
                self.append(&code);
            },
            MdAst::Text { content } => {
                self.append(&content);
            },
            MdAst::Paragraph { children } => {
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

    pub fn lisp_fragments(&self) -> Vec<LispFragments> {
        self.frags.clone()
    }

}

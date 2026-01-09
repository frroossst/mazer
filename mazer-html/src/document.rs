use std::{collections::BTreeMap, rc::Rc};

use mazer_lisp::parser::Parser;
use mazer_types::Environment;
use mazer_parser::MdAst;
use mazer_render::{ToMathML, format_mathml_with_env};
use mazer_types::LispAST;



enum FontKind {
    Bold,
    Italic,
    Underline,
    Strikethrough,
}

#[derive(Debug, Clone)]
pub enum DocAst {
    Html(Rc<str>),
    Eval(LispAST),
    Show(LispAST),
}

// MdAst >| DocAst >| String
pub struct Document {
    head: String,
    body: Vec<DocAst>,
    nodes: Vec<MdAst>,
}

impl Document {

    pub fn new(nodes: Vec<MdAst>) -> Self {

        let head = "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"UTF-8\"><link rel=\"icon\" href=\"data:image/svg+xml,<svg xmlns=%22http://www.w3.org/2000/svg%22 viewBox=%220 0 32 32%22><text y=%2232%22 font-size=%2232%22>🍁</text></svg>\"><script src=\"https://cdn.jsdelivr.net/npm/@arborium/arborium/dist/arborium.iife.js\" data-theme=\"github-light\" data-selector=\"pre code\"></script></head>";

        Document {
            head: String::from(head),
            body: Vec::new(),
            nodes,
        }
    }

    pub fn body(&self) -> Vec<DocAst> {
        self.body.clone()
    }

    pub fn build(&mut self) {
        self.append(DocAst::Html("<body>".into()));
        for node in &self.nodes.clone() {
            self.append_node(node.clone());
        }
        self.append(DocAst::Html("</body>".into()));
    }

    pub fn fragments(&self) -> BTreeMap<String, LispAST> {
        // Only return Eval blocks for evaluation - Show blocks are formatted symbolically
        self.body
            .iter()
            .filter_map(|content| match content {
                DocAst::Eval(ast) => {
                    let key = format!("{:?}", ast);
                    Some((key, ast.clone()))
                },
                _ => None,
            })
            .collect()
    } 

    /// Inject evaluated results for Eval blocks.
    /// Show blocks are handled separately via format_show_blocks().
    pub fn inject(&mut self, results: &BTreeMap<String, LispAST>) {
        for content in &mut self.body {
            match content {
                DocAst::Eval(e) => {
                    let key = format!("{:?}", e);
                    if results.get(&key).is_some() {
                        // Eval blocks execute for side effects only, don't display
                        *content = DocAst::Html("".into());
                    }
                },
                // Show blocks are now handled by format_show_blocks, not here
                _ => {},
            }
        }
    }
    
    /// Format all Show blocks using the symbolic Show formatter.
    /// This preserves the symbolic structure and converts to MathML without evaluation.
    /// Call this after inject() to format show blocks with the environment from evaluation.
    // Format show blocks symbolically using the environment from evaluation
    // This allows show blocks to use variables defined in eval blocks
    pub fn fmt(&mut self, env: &Environment) {
        for content in &mut self.body {
            if let DocAst::Show(ast) = content {
                let formatted = format_mathml_with_env(ast, Some(env));
                // Wrap in <math> tags for proper MathML rendering
                let mathml = format!("<math display=\"inline\">{}</math>", formatted);
                *content = DocAst::Html(mathml.into());
            }
        }
    }

    pub fn output(&self) -> String {
        let mut html = String::with_capacity(1024);
        html.push_str(&self.head);

        for n in &self.body {
            match n {
                DocAst::Html(content) => {
                    html.push_str(content);
                },
                DocAst::Eval(_e) => {
                    // NOTE: eval is expected to be in its final transformed state
                    unreachable!("Interpreter should have processed all Eval blocks before output");
                },
                DocAst::Show(s) => {
                    // Fallback if format_show_blocks wasn't called - use ToMathML
                    let s: String = s.to_mathml(); 
                    let mathml = format!("<math display=\"inline\">{}</math>", s);
                    html.push_str(&mathml);
                },
            }
        }
        html.push_str("</html>");
        html
    }

    fn append(&mut self, content: DocAst) {
        self.body.push(content);
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
                let mut p = Parser::new(&code);
                let r = p.parse().unwrap_or(LispAST::Error("<<failed to parse>>".into()));

                let dast = DocAst::Eval(r);
                self.append(dast);
            },
            MdAst::ShowBlock { code } => {
                let mut p = Parser::new(&code);
                let r = p.parse().unwrap_or(LispAST::Error("<<failed to parse>>".into()));

                let dast = DocAst::Show(r);
                self.append(dast);
            },
            MdAst::Text { content } => {
                let dast = if content == "\n" {
                    DocAst::Html("<br/>".into())
                } else {
                    // Replace newlines within text with <br/> tags
                    let html_content = content.replace("\n", "<br/>");
                    DocAst::Html(html_content.into())
                };
                self.append(dast);
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

        let dast = DocAst::Html(header_html.into());
        self.append(dast);
    }

    #[inline]
    fn append_unordered_list(&mut self, items: Vec<String>) {
        let mut list_html = String::from("<ul>");
        for item in items {
            list_html.push_str(&format!("<li>{}</li>", item));
        }
        list_html.push_str("</ul>");

        let dast = DocAst::Html(list_html.into());
        self.append(dast);
    }

    #[inline]
    fn append_checkbox(&mut self, text: String, checked: bool) {
        let checkbox_html = if checked {
            format!("<input type=\"checkbox\" checked disabled> {}<br>", text)
        } else {
            format!("<input type=\"checkbox\" disabled> {}<br>", text)
        };

        let dast = DocAst::Html(checkbox_html.into());
        self.append(dast);
    }

    #[inline]
    fn append_blockquote(&mut self, content: String) {
        let blockquote_html = format!("<blockquote>{}</blockquote>", content);

        let dast = DocAst::Html(blockquote_html.into());
        self.append(dast);
    }

    #[inline]
    fn append_spoiler(&mut self, content: String) {
        let begin = "<span style=\"background:#2f3136;color:transparent;border-radius:3px;padding:0 4px;cursor:pointer\" onmouseover=\"this.style.color='#dcddde'\" onmouseout=\"this.style.color='transparent'\">";
        let end = "</span>";
        let spoiler_html = format!("{}{}{}", begin, content, end);

        let dast = DocAst::Html(spoiler_html.into());
        self.append(dast);
    }

    #[inline]
    fn append_link(&mut self, text: String, url: String) {
        // always open in new tab
        let link_html = format!("<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">{}</a>", url, text);

        let dast = DocAst::Html(link_html.into());
        self.append(dast);
    }

    #[inline]
    fn append_codeblock(&mut self, code: String, language: Option<String>) {
        let lang_html = format!("<pre><code class=\"language-{}\">{}</code></pre>", language.unwrap_or_default(), code);

        let dast  = DocAst::Html(lang_html.into());
        self.append(dast);
    }

    #[inline]
    fn append_inline_code(&mut self, code: String) {
        let inline_code_html = format!("<code>{}</code>", code);

        let dast = DocAst::Html(inline_code_html.into());
        self.append(dast);
    }

    #[inline]
    fn append_text(&mut self, text: String, kind: FontKind) {
        let formatted_text = match kind {
            FontKind::Bold => format!("<strong>{}</strong>", text),
            FontKind::Italic => format!("<em>{}</em>", text),
            FontKind::Underline => format!("<u>{}</u>", text),
            FontKind::Strikethrough => format!("<s>{}</s>", text),
        };

        let dast = DocAst::Html(formatted_text.into());
        self.append(dast);
    }

    #[inline]
    fn append_page_separator(&mut self) {
        let dast = DocAst::Html("<hr/>".into());
        self.append(dast);
    }


}

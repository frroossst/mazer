use crate::tokenizer::MarkdownTag;


#[derive(Debug)]
pub struct Document {
    top: String,
    body: Vec<String>,
    btm: String,
    let_idx: Vec<usize>,
}

impl Document {
    pub fn new(title: &str) -> Self {

        let html = format!("<!DOCTYPE html>\n<html lang=\"en-US\">\n");
        let meta = r#"
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            "#;
        let style = r#"
        <style>
            blockquote {
                background-color: #0d1117;
                border-left: 4px solid #58a6ff;
                color: #c9d1d9;
                margin: 20px 0;
                padding: 10px 20px;
                position: relative;
            }

            blockquote p {
                margin: 0;
                color: #8b949e;
                }
            img {
                width: 600px;
                height: 400px;
            }
            /* GitHub Style Dark Mode CSS */

            body {
                background-color: #0d1117;
                color: #c9d1d9;
                font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif, "Apple Color Emoji", "Segoe UI Emoji";
                margin: 0;
                padding: 0;
            }

        </style>
        "#;
        let title = format!("<head>\n<title>{}</title>\n</head><body>", title);

        let end = format!("</body></html>");

        Document {
            top: html + &meta +&style + &title,
            body: Vec::new(),
            btm: end,
            let_idx: Vec::new(),
        }

    }

    pub fn output(&self) -> String {
        let mut body = String::new();
        for content in self.body.clone() {
            body.push_str(&content);
        }
        self.top.clone() + &body + &self.btm
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

    pub fn append_newlne(&mut self) {
        self.body.push(String::from("<br>\n"));
    }

    pub fn add_markdown(&mut self, tag: MarkdownTag) {
        match tag {
            MarkdownTag::Header(hk, content) => {
                // consume # from the content
                // if 0 then it is a h1 tag
                // if 1 then it is a h2 tag
                // if 2 then it is a h3 tag
                let content = content;
                let num_hashes: usize = hk.into();

                self.append_wrapped(&format!("h{}", num_hashes), &content);
            },
            MarkdownTag::LineSeparator => {
                self.append_void("hr");
            },
            MarkdownTag::Checkbox(state, content) => {
                if state {
                    self.append_wrapped_with_attr("input", "type=\"checkbox\" checked disabled", &content[5..]);
                } else {
                    self.append_wrapped_with_attr("input", "type=\"checkbox\" disabled", &content[5..]);
                }
                self.append_newlne();
            },
            MarkdownTag::BulletPoint(content) => {
                let content = content[2..].to_string();
                let list_elem = format!("• {}", content);
                self.append_wrapped("div", &list_elem);
                self.append_newlne();
            },
            MarkdownTag::Blockquote(content) => {
                self.append_wrapped("blockquote", &content[1..]);
                self.append_newlne();
            },
            MarkdownTag::Link(_, text, link) => {
                let is_image = link.starts_with("!");

                // get content within [ ]
                let display = link.split('[').collect::<Vec<&str>>()[1].split(']').collect::<Vec<&str>>()[0];
                // get content within ( )
                let link = link.split('(').collect::<Vec<&str>>()[1].split(')').collect::<Vec<&str>>()[0];

                if is_image {
                    self.append_wrapped_with_attr("img", &format!("src={} alt={}", link, display), "");
                } else{
                    self.append_wrapped_with_attr("a", &format!("href={} target=\"_blank\"", link), display);
                }
                self.append_newlne();
            },
            _ => { unimplemented!() },
        }
    }

    fn process_inline_markdown(&self, content: &str) -> String {
        // replace **text** with <b>text</b>
        // replace *text* with <i>text</i>

        // find the index of the first occurence of **
        // replace it with <b>
        // find the index of the second occurence of **
        // replace it with </b>
        // alternate and loop until no more
        let mut content = content.to_string();

        let mut open = false;
        while let Some(_start) = content.find("**") {
            if open {
                content = content.replacen("**", "</b>", 1);
            } else {
                content = content.replacen("**", "<b>", 1);
            }
            open = !open;
        }

        while let Some(_start) = content.find("*") {
            if open {
                content = content.replacen("*", "</i>", 1);
            } else {
                content = content.replacen("*", "<i>", 1);
            }
            open = !open;
        }

        return content;
    }

    pub fn add_text(&mut self, text: String) {
        let text = self.process_inline_markdown(&text);
        self.append_wrapped("p", &text);
    }

    pub fn add_let(&mut self, expr: String) {

        let let_idx = self.body.len();

        self.append_wrapped("pre", &expr);
        self.append_newlne();

        self.let_idx.push(let_idx);
    }
}

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
}

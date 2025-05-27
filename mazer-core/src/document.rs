use crate::{
    parser::MathML,
    tokenizer::{Emphasis, LinkKind, MarkdownTag},
    wrap_mathml,
};

#[derive(Debug)]
pub struct Document {
    head: String,
    title: String,
    body: Vec<String>,
}

impl Document {
    pub fn new(title: &str) -> Self {
        Document {
            head: format!(
                "<!DOCTYPE html lang=\"en\">\n<html>\n<head>\n<meta charset=\"utf-8\">\n</head>\n"
            ),
            title: format!("<title> Mazer - {} </title>", title),
            body: Vec::new(),
        }
    }

    pub fn title(&self) -> String {
        self.title.clone()
    }

    pub fn output(&self) -> String {
        let mut body = String::new();

        body.push_str(&self.head);
        body.push_str(&self.title);

        body.push_str("<body>\n");

        for content in self.body.clone() {
            body.push_str(&content);
        }

        body.push_str("</body>\n");

        body
    }

    pub fn stylistic_output(&self) -> String {
        let mut body = String::new();

        let before = r#"
        <!DOCTYPE html>
<html lang="en-US">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <link rel="icon" href="data:image/svg+xml,<svg xmlns=%22http://www.w3.org/2000/svg%22 viewBox=%220 0 32 32%22><text y=%2232%22 font-size=%2232%22>🍁</text></svg>">
    <style>
        /* General styles */
        h1, h2, h3 {
            margin: 0;
            padding: 0,
        }

        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif, "Apple Color Emoji", "Segoe UI Emoji";
            margin: 0;
            padding: 0;
            transition: background-color 0.3s, color 0.3s;
        }

        /* Dark mode styles */
        body.dark-mode {
            background-color: #0d1117;
            color: #c9d1d9;
        }

        body.dark-mode blockquote {
            padding: 10px 20px;
            margin: 20px 0;
            border-left: 3px solid #8b949e;
            background-color: #161b22;
            color: #c9d1d9;
        }

        code.dark-mode {
            background-color: #0d1117; 
            color: #c9d1d9; 
            padding: 10px 20px;
            overflow-x: auto;
            border-radius: 15px;
        }

        .inline-code {
            font-family: monospace;
            background-color: #0d1117;
            color: #c9d1d9;
            display: inline;
            border-radius: 3px;
            padding: 2px 4px;
        }

        img {
            width: 600px;
            height: 400px;
        }

        /* Light mode styles */
        body.light-mode {
            background-color: #ffffff;
            color: #000000;
        }

        body.light-mode h1, body.light-mode h2, body.light-mode h3 {
            color: #333;
        }

        body.light-mode blockquote {
            padding: 10px 20px;
            margin: 20px 0;
            border-left: 3px solid #444d56;
            background-color: #f6f8fa;
            color: #24292e;
        }

        body.light-mode code {
            background-color: #f9f9f9;
            color: #333;
        }

        body.light-mode .inline-code {
            background-color: #f9f9f9;
            color: #333;
            display: inline;
        }

        /* Toggle switch styles */
        .toggle-container {
            position: fixed;
            top: 10px;
            right: 10px;
            z-index: 1000;
            display: flex;
            flex-direction: column;
            align-items: flex-end;
        }

        .toggle-switch {
            position: relative;
            display: inline-block;
            width: 60px;
            height: 34px;
            margin-bottom: 10px;
            margin-right: 20px;
        }

        .toggle-switch input {
            opacity: 0;
            width: 0;
            height: 0;
        }

        .slider {
            position: absolute;
            cursor: pointer;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            background-color: #ccc;
            transition: .4s;
            border-radius: 34px;
        }

        .slider:before {
            position: absolute;
            content: "";
            height: 26px;
            width: 26px;
            left: 4px;
            bottom: 4px;
            background-color: white;
            transition: .4s;
            border-radius: 50%;
        }

        input:checked + .slider {
            background-color: #2196F3;
        }

        input:checked + .slider:before {
            transform: translateX(26px);
        }

        .app-info {
            color: #8b949e;
            font-size: 0.9em;
            text-align: center;
            margin-right: 20px;
        }

        .content {
            max-width: 1200px;
            margin: 20px 50px;
            padding: 0 20px;
            line-height: 1.6;
        }
    </style>
    <style>
        .eval-result {
          display: inline-block;
          padding: 2px 6px;
          border: 1px solid #888;
          border-radius: 4px;
          background-color: #f8f8f8;
          cursor: pointer;
          font-family: monospace;
          transition: background-color 0.2s ease;
          color: #000000;
        }

        .eval-result.dark-mode {
          background-color: #0d1117;
          color: #ffffff;
        }
        
        .eval-result:hover {
          background-color: #e0e0e0;
        }
        
        .hover-hint {
          visibility: hidden;
          position: absolute;
          background: #333;
          color: #fff;
          padding: 4px 8px;
          border-radius: 4px;
          font-size: 0.9em;
          white-space: nowrap;
          transform: translateY(-30px);
          opacity: 0;
          transition: opacity 0.2s ease, transform 0.2s ease;
        }
        
        .eval-container {
          position: relative;
          display: inline-block;
        }
      
        .eval-container:hover .hover-hint {
          visibility: visible;
          opacity: 1;
          transform: translateY(-35px);
        }
      </style>
</head>
<script>
        function toggleTheme() {
            const body = document.body;
            body.classList.toggle('dark-mode');
            body.classList.toggle('light-mode');
        }

        // Automatically set the toggle switch based on current theme
        document.addEventListener('DOMContentLoaded', (event) => {
            const isDarkMode = document.body.classList.contains('dark-mode');
            document.getElementById('themeToggle').checked = !isDarkMode;
        });
</script>
<body class="dark-mode">
    <div class="toggle-container">
        <label class="toggle-switch">
            <input type="checkbox" id="themeToggle" onclick="toggleTheme()">
            <span class="slider"></span>
        </label>
        <div class="app-info">
            <div>🍁Mazer🌰</div>
        </div>
    </div>
        "#;

        body.push_str(before);

        for content in self.body.clone() {
            body.push_str(&content);
        }

        body.push_str("</body>\n");
        body.push_str("</html>\n");

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
        self.body
            .push(format!("<{} {}>{}</{}>", tag, attr, content, tag));
    }

    pub fn append_wrapped(&mut self, tag: &str, content: &str) {
        self.body.push(format!("<{}>{}</{}>", tag, content, tag));
    }

    pub fn append_newline(&mut self) {
        self.body.push(String::from("<br>\n"));
    }

    pub fn append_math_ml(&mut self, content: MathML) {
        self.append_wrapped("math", &wrap_mathml!(content.string()));
        self.append_newline();
    }

    pub fn append_raw_math_ml(&mut self, content: MathML) {
        if content.string().starts_with("<math") {
            panic!("Invalid MathML content");
        }
        self.append(content.string());
    }

    pub fn append_code(&mut self, content: &str) {
        self.append_wrapped("code", &content);
    }

    pub fn append_evaluation(&mut self, expression: &str, result: &str) {
        // <div class="eval-container">
        //      <span class="eval-result">result</span>
        //      <span class="hover-hint">expression</span>
        // </div>
        let content = format!(
            "<div class=\"eval-container\"><span class=\"eval-result\">{}</span><span class=\"hover-hint\">{}</span></div>",
            result, expression
        );

        self.append(content);
    }

    pub fn append_text(&mut self, emphasis: Option<Emphasis>, content: &str) {
        if emphasis.is_none() {
            self.append(content.to_string());
        } else {
            match emphasis.unwrap() {
                Emphasis::Bold => {
                    self.append_wrapped("b", content);
                }
                Emphasis::Italic => {
                    self.append_wrapped("i", content);
                }
                Emphasis::Strikethrough => {
                    self.append_wrapped("s", content);
                }
            }
        }
    }

    pub fn add_markdown(&mut self, markdown: MarkdownTag) {
        match markdown {
            MarkdownTag::Header(level, content) => {
                let header_count: usize = level.into();
                self.append_wrapped(&format!("h{}", header_count), &content);
            }
            MarkdownTag::LineSeparator => {
                self.append_void("hr");
            }
            MarkdownTag::Table(headers, rows) => {
                if headers.is_empty() {
                    // skip
                    return;
                }

                let table_header = headers
                    .iter()
                    .map(|header| format!("<th>{}</th>", header))
                    .collect::<Vec<String>>()
                    .join("");

                let table_rows = rows
                    .iter()
                    .map(|row| {
                        let row_content = row
                            .iter()
                            .map(|cell| format!("<td>{}</td>", cell))
                            .collect::<Vec<String>>()
                            .join("");
                        format!("<tr>{}</tr>", row_content)
                    })
                    .collect::<Vec<String>>()
                    .join("");

                self.append_wrapped_with_attr(
                    "table",
                    "border=1",
                    &format!(
                        "<thead>{}</thead><tbody>{}</tbody>",
                        table_header, table_rows
                    ),
                );

                self.append_newline();
            }
            MarkdownTag::Checkbox(state, content) => {
                let checked = if state { "checked" } else { "" };

                self.append_wrapped_with_attr(
                    "input",
                    &format!("type=\"checkbox\" disabled {}", checked),
                    "",
                );

                if state {
                    self.append_text(Some(Emphasis::Strikethrough), &content);
                } else {
                    self.append_text(None, &content);
                }

                self.append_newline();
            }
            MarkdownTag::BulletPoint(content) => {
                self.append_wrapped("li", &content);
            }
            MarkdownTag::Blockquote(content) => {
                self.append_wrapped("blockquote", &content);
            }
            MarkdownTag::CodeBlock(content) => {
                let content = content.replace("\n", "<br>");
                self.append_wrapped("pre", &content);
            }
            MarkdownTag::Link(kind, display, link) => match kind {
                LinkKind::Image => {
                    self.append_newline();
                    self.append_wrapped_with_attr(
                        "img",
                        &format!("src=\"{}\" alt=\"{}\"", link, display),
                        "",
                    );
                }
                LinkKind::Hyperlink => {
                    self.append_wrapped_with_attr(
                        "a",
                        &format!("href=\"{}\" target=\"_blank\" ", link),
                        &display,
                    );
                }
            },
        }
    }
}

use crate::{lexer::Lexer, vm::VirtualMachine};


#[derive(Debug)]
pub enum LineTag {
    Text(String),
    Let(String),
    Markdown(MarkdownTag),
}

#[derive(Debug)]
pub enum MarkdownTag {
    Header(String),
    LineSeparator,
    Checkbox(bool, String),
    BulletPoint(String),
    Blockquote(String),
    Link(String),
}

#[derive(Debug)]
pub struct Parser {
    src: Vec<String>,
    pos: usize,
    // symbols: MaybeSolveable
    ctx: Lexer,
    // executable bytecode
    exe: VirtualMachine,
}

impl Parser {

    pub fn new(src: String) -> Self {
        let src: Vec<String> = src.split_terminator('\n').map(|s| s.to_string()).collect();
        let mut lines: Vec<String> = Vec::with_capacity(src.len());

        let mut consume = false;
        let mut curr_line = String::new();

        // each line is separated by a newline character
        // except if the line starts with let <symbol> = <value> then it ends with a semicolon
        for idx in 0..src.len() {
            let line = src[idx].trim();
            if line.trim().starts_with("let") {
                consume = true;
            }

            if consume {
                curr_line.push_str(line);
                if line.ends_with(";") {
                    lines.push(curr_line);
                    curr_line = String::new();
                    consume = false;
                }
            } else {
                lines.push(line.to_string());
            }
        }
        
        Parser { 
            src: lines,
            pos: 0,
            ctx: Lexer::new(),
            exe: VirtualMachine::new(),
        }
    }

    fn has_comment(&self, line: &str) -> bool {
        // but is not a link
        line.contains("//") && !line.contains("http")
    }

    fn is_markdown(&self, line: &str) -> bool {
        // header tags
        line.starts_with("#")
        || line.starts_with("##")
        || line.starts_with("###")
        // line separator
        || line.starts_with("===")
        // checkboxes
        || line.starts_with("- [ ]")
        || line.starts_with("- [x]")
        // bullet points
        || line.starts_with("-")
        // blockquotes
        || line.starts_with(">")
        // code blocks
        // || line.starts_with("```")
        // link
        || ( line.contains("[") && line.contains("]") && line.contains("(") && line.contains(")") )
    }

    fn get_markdown_tag(&self, line: &str) -> Option<MarkdownTag> {
        if line.starts_with("#") {
            Some(MarkdownTag::Header(line.to_string()))
        } else if line.starts_with("===") {
            Some(MarkdownTag::LineSeparator)
        } else if line.starts_with("- [ ]") {
            Some(MarkdownTag::Checkbox(false, line.to_string()))
        } else if line.starts_with("- [x]") {
            Some(MarkdownTag::Checkbox(true, line.to_string()))
        } else if line.starts_with("-") {
            Some(MarkdownTag::BulletPoint(line.to_string()))
        } else if line.starts_with(">") {
            Some(MarkdownTag::Blockquote(line.to_string()))
        } else if line.contains("[") && line.contains("]") && line.contains("(") && line.contains(")"){
            Some(MarkdownTag::Link(line.to_string()))
        } else {
            None
        }
    }

    fn is_let_expr(&self, line: &str) -> bool {
        line.starts_with("let")
    }

    pub fn next_tagged(&mut self) -> Option<LineTag> {
        if self.pos >= self.src.len() {
            return None;
        }

        let mut line = self.src[self.pos].trim();

        let is_lexable = self.ctx.is_lexable(&line);
        if is_lexable {
            self.ctx.lex(&line)
        }

        if self.has_comment(line) {
            // remove comments
            let idx = line.find("//").unwrap();
            line = line[..idx].trim();
        }

        if self.is_markdown(line) {
            self.pos += 1;
            let md_tag = self.get_markdown_tag(line).expect("Should have a markdown tag as is_markdown returned true");
            return Some(LineTag::Markdown(md_tag));
        } else if self.is_let_expr(line) {
            self.pos += 1;
            return Some(LineTag::Let(line.to_string()));
        } else {
            self.pos += 1;
            return Some(LineTag::Text(line.to_string()));
        }
    }

    pub fn resolve_ctx(&mut self) {
        // TODO: resolve ctx
        // 1. Substitute fmt() with the HTML value in ctx
        // 2. Substitute eval() if VM is implemented
        for (k, v) in self.ctx.symbols() {
            println!("{} = {:?}", k, v);
        }
    }
}

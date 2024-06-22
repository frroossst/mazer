use unicode_segmentation::UnicodeSegmentation;

use crate::pretty_err::{DebugContext, ErrorKind};


#[derive(Debug, Clone)]
pub enum MarkdownTag {
    Header(HeaderKind, String),
    LineSeparator,
    Checkbox(bool, String),
    BulletPoint(String),
    Blockquote(String),
    CodeBlock(String),
    Link(LinkKind, String, String),
}

#[derive(Debug, Clone)]
pub enum LinkKind {
    Image,
    Hyperlink,
}

#[derive(Debug, Clone)]
pub enum HeaderKind {
    H1,
    H2,
    H3
}

impl Into<usize> for HeaderKind {
    fn into(self) -> usize {
        match self {
            HeaderKind::H1 => 1,
            HeaderKind::H2 => 2,
            HeaderKind::H3 => 3,
        }
    }

}

impl From<usize> for HeaderKind {
    fn from(val: usize) -> Self {
        match val {
            1 => { HeaderKind::H1 },
            2 => { HeaderKind::H2 },
            3 => { HeaderKind::H3 },
            _ => { HeaderKind::H1 },
        }
    }
}

#[derive(Debug, Clone)]
pub enum Token {
    LetExpr(String, String),
    Fn(FnKind, String),
    Literal(String),
    Text(Option<Emphasis>, String),
    Comment(String),
    Markdown(MarkdownTag),
    Newline,
}

#[derive(Debug, Clone)]
pub enum FnKind {
    Fmt,
    Eval,
}

impl Into<String> for FnKind {
    fn into(self) -> String {
        match self {
            FnKind::Fmt => "fmt".to_string(),
            FnKind::Eval => "eval".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Emphasis {
    Bold,
    Italic,
    Strikethrough,
}

#[derive(Debug)]
pub struct Tokenizer {
    src: Vec<String>,
    pos: usize,
    line: usize,
    max: usize,
    ctx: DebugContext,
}

impl Tokenizer {

    pub fn new(src: String, ctx: DebugContext) -> Self {

        let uni_vec = UnicodeSegmentation::graphemes(src.as_str(), true)
                                    .collect::<Vec<&str>>()
                                    .iter()
                                    .map(|&x| x.to_string())
                                    .collect::<Vec<String>>();

        let max = uni_vec.len();
        Tokenizer {
            src: uni_vec,
            pos: 0,
            line: 0,
            max,
            ctx,
        }
    }

    fn panic(&mut self, err: ErrorKind) -> ! {
        // need to calculate what character the error is at
        // go through all the tokens, count \n and then calculate
        // the position of the error
        let err_pos = self.src.iter().take(self.pos).fold(0, |acc, x| {
            if x == "\n" {
                acc + 1
            } else {
                acc
            }
        });

        // get the line where the error is and then get the line
        // get the position and then go back on the iterator to find a newline
        // and go forward to find the a newline and extract an error line
        let mut start = 0;
        let mut end = 0;
        let mut line = 0;
        for (i, x) in self.src.iter().enumerate() {
            if x == "\n" {
                if line == err_pos {
                    end = i;
                    break;
                }
                start = i;
                line += 1;
            }
        }

        let err_line = self.src[start..end].join("");
        dbg!(&err_line);

        self.ctx.set_source_code(err_line);
        self.ctx.set_position(self.line, err_pos);
        self.ctx.set_error(err);
        self.ctx.panic();
    }

    fn char(&mut self) -> String {
        if self.pos >= self.max {
            // [ERROR]
            self.panic(ErrorKind::AbruptAdieu(format!("Reached the end of file looking for position {}", self.pos)));
        }
        self.src[self.pos].clone()
    }

    fn peek(&mut self) -> String {
        self.src[self.pos + 1].clone()
    }

    // peeks the char after the next char
    fn peek_n(&mut self, n: usize) -> String {
        self.src[self.pos + n].clone()
    }

    fn advance_char(&mut self) {
        if self.pos >= self.max {
            return;
        }
        self.pos += 1;
    }

    fn must_consume(&mut self, c: &str) {
        let curr = self.char();
        // [ERROR]
        if curr != c {
            self.panic(ErrorKind::BrokenExpectations(format!("Expected '{}' but found '{}'", c, curr)));
        }
        self.advance_char();
    }

    fn consume_whitespace(&mut self) {
        // keep moving forward if current string is made up of
        // whitespaces
        while self.char().trim().is_empty() {
            self.advance_char();
        }
    }

    fn consume_until_not(&mut self, c: &str) -> String {
        let start= self.pos;
        while self.char() == c {
            self.pos += 1;
        }

        self.src[start..self.pos].join("")
    }

    fn consume_till(&mut self, c: &str) -> String {
        let start = self.pos;
        while self.char() != c {
            self.pos += 1;
        }
        self.src[start..self.pos].join("")
    }

    fn consume_line(&mut self) -> String {
        self.consume_till("\n")
    }

    // helper for consuming nested parenthesis
    fn consume_nested_parenthesis(&mut self) -> String {
        let mut store = String::from(self.char());
        let mut stack: Vec<()> = Vec::new();
        stack.push(());

        while !stack.is_empty() {
            self.advance_char();
            let curr = self.char();
            if curr == "(" {
                stack.push(());
                store.push_str(&curr);
            } else if curr == ")" {
                stack.pop();
                store.push_str(&curr);
            } else {
                store.push_str(&curr.to_string());
            }
        }

        if stack.len() != 0 {
            // [ERROR]
            self.panic(ErrorKind::LonelyParenthesis("Unmatched parenthesis".to_string()));
        }

        return store;
    }

    pub fn next_line(&mut self) -> Option<Vec<Token>> {
        self.line += 1;
        if self.pos >= self.max {
            return None;
        }

        let mut tokens: Vec<Token> = Vec::new();
        while let Some(tok) = self.next_token() {
            tokens.push(tok);
        }
        self.advance_char();

        if tokens.is_empty() {
            return Some(vec![Token::Newline]);
        }

        Some(tokens)
    }

    fn next_token(&mut self) -> Option<Token> {
        if self.pos >= self.max || self.char() == "\n" {
            return None;
        }

        let curr_tok = self.char();

        // consume comments
        if curr_tok == "/" && self.peek() == "/" {
            self.advance_char();
            self.advance_char();
            let comment = self.consume_line();
            let comment = comment.trim();
            return Some(Token::Comment(comment.to_string()));
        // literals
        } else if curr_tok == "\"" {
            self.advance_char();
            let literal = self.consume_till("\"").to_string();
            self.must_consume("\"");

            return Some(Token::Literal(literal));
        // let statements
        } else if curr_tok == "l" && self.peek() == "e" && self.peek_n(2) == "t" {

            self.advance_char();
            self.advance_char();
            self.advance_char();

            if self.char() != " " {
                // [ERROR]
                self.panic(ErrorKind::GrammarGoblin("Let statement should be followed by a space".to_string()));
            }

            let var = self.consume_till("=").trim().to_string();
            // [ERROR] 
            // TODO: check if variable name is valid
            if var.is_empty() {
                self.panic(ErrorKind::NamelessNomad("Variable name cannot be empty".to_string()));
            }

            self.must_consume("=");
            // self.consume_whitespace();

            let mut val = self.consume_till(";").trim().to_string();
            self.must_consume(";");
            val.push_str(";");

            return Some(Token::LetExpr(var, val));
        // fmt calls
        } else if curr_tok == "f" && self.peek() == "m" && self.peek_n(2) == "t" {
            self.advance_char();
            self.advance_char();
            self.advance_char();
            self.must_consume("(");

            // the body expression may have parenthesis in it, so need to maintain a stack and 
            // consume until the stack is empty
            let fmt = self.consume_nested_parenthesis().trim().to_string();

            self.must_consume(")");

            return Some(Token::Fn(FnKind::Fmt, fmt));
        // eval calls
        } else if curr_tok == "e" && self.peek() == "v" && self.peek_n(2) == "a" && self.peek_n(3) == "l" {
            self.advance_char();
            self.advance_char();
            self.advance_char();
            self.advance_char();

            self.must_consume("(");
            let eval = self.consume_nested_parenthesis().trim().to_string();

            self.must_consume(")");

            return Some(Token::Fn(FnKind::Eval, eval));
        // headers
        } else if curr_tok == "#" {
            let hash_count = self.consume_until_not("#").len();

            let heading = self.consume_line();
            let heading = heading.trim();

            let header_kind: HeaderKind = hash_count.into();

            return Some(
                Token::Markdown(
                    MarkdownTag::Header(header_kind, heading.to_string())
                )
            );
        // blockquote
        } else if curr_tok == ">" {
            self.advance_char();
            let blockquote = self.consume_line();
            let blockquote= blockquote.trim();
            return Some(
                Token::Markdown(
                    MarkdownTag::Blockquote(blockquote.to_string())
                )
            );
        // bullets or checkboxes 
        } else if curr_tok == "-" {
            self.advance_char();
            self.consume_whitespace();

            let is_bullet = self.char() != "[";
            if is_bullet {
                let bullet = self.consume_line();
                let bullet = bullet.trim();
                return Some(
                    Token::Markdown(
                        MarkdownTag::BulletPoint(bullet.to_string())
                    )
                );
            }

            self.advance_char();
            let is_checked = self.char() == "x";

            self.advance_char();
            self.must_consume("]");
            self.consume_whitespace();

            let checkbox = self.consume_line();
            let checkbox = checkbox.trim();
            return Some(
                Token::Markdown(
                    MarkdownTag::Checkbox(is_checked, checkbox.to_string())
                )
            );
        // line separator
        } else if curr_tok == "=" {

            self.consume_until_not("=");
            if self.consume_line().trim().len() > 0 {
                self.panic(ErrorKind::GrammarGoblin("Line separator should contain only '=' characters".to_string()));
            }

            return Some(
                Token::Markdown(
                    MarkdownTag::LineSeparator
                )
            );
        // consume links
        } else if (curr_tok == "!" && self.peek() == "[") || curr_tok == "[" {
            let is_image = curr_tok == "!";
            if is_image {
                self.advance_char();
            }
            self.must_consume("[");
            let text = self.consume_till("]").to_string();
            self.must_consume("]");
            self.must_consume("(");
            let link = self.consume_till(")").to_string();
            self.must_consume(")");

            return Some(Token::Markdown(
                MarkdownTag::Link(
                    if is_image { LinkKind::Image } else { LinkKind::Hyperlink },
                    text,
                    link,
                )
            ));
        // code blocks
        } else if curr_tok == "`" {
            // check if inline code block or code block
            let code: String;
            if self.peek() == "`" {
                self.must_consume("`");
                self.must_consume("`");
                self.must_consume("`");

                self.consume_whitespace();
                code = self.consume_till("`").to_string();

                self.must_consume("`");
                self.must_consume("`");
                self.must_consume("`");
            } else {
                self.must_consume("`");
                code = self.consume_till("`").trim().to_string();
                self.must_consume("`");
            }

            return Some(Token::Markdown(
                MarkdownTag::CodeBlock(code)
            ));
        // bold
        } else if curr_tok == "*" {
            if self.peek() == "*" {
                self.advance_char();
                self.advance_char();
                let text = self.consume_till("*").to_string();
                self.must_consume("*");
                self.must_consume("*");

                return Some(Token::Text(Some(Emphasis::Bold), text));
            } else {
                self.advance_char();
                let text = self.consume_till("*").to_string();
                self.must_consume("*");

                return Some(Token::Text(Some(Emphasis::Italic), text));
            }
        // strikethrough
        } else if curr_tok == "~" {
            self.advance_char();
            let text = self.consume_till("~").to_string();
            self.must_consume("~");

            return Some(Token::Text(Some(Emphasis::Strikethrough), text));
        // text
        } else {
            let text = curr_tok.to_string();
            self.advance_char();
            return Some(Token::Text(None, text));
        }
    }

    /// Takes in a collection of tokens and tries
    /// to compacts the repeated text tokens into one
    /// only if they have the same emphasis
    pub fn compact(tokens: Vec<Token>) -> Vec<Token> {
        let mut compacted = Vec::new();
        let mut iter = tokens.iter().peekable();

        while let Some(token) = iter.next() {
            match token {
                Token::Text(emphasis, text) => {
                    let mut combined_text = text.clone();
                    while let Some(&Token::Text(next_emphasis, next_text)) = iter.peek() {
                        if emphasis == next_emphasis {
                            combined_text.push_str(next_text);
                            iter.next(); // consume the token
                        } else {
                            break;
                        }
                    }
                    compacted.push(Token::Text(emphasis.clone(), combined_text));
                }
                _ => compacted.push(token.clone()),
            }
        }

        compacted
        }

} 

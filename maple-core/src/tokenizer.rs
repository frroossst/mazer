use crate::pretty_err::{DebugContext, PrettyErrContext, PrettyErrKind};

#[derive(Debug)]
pub enum MarkdownTag {
    Header(HeaderKind, String),
    LineSeparator,
    Checkbox(bool, String),
    BulletPoint(String),
    Blockquote(String),
    CodeBlock(String),
    Link(LinkKind, String, String),
}

#[derive(Debug)]
pub enum LinkKind {
    Image,
    Hyperlink,
}

#[derive(Debug)]
pub enum HeaderKind {
    H1,
    H2,
    H3
}

impl Into<usize> for HeaderKind {
    fn into(self) -> usize {
        match self {
            HeaderKind::H1 => { 1 },
            HeaderKind::H2 => { 2 },
            HeaderKind::H3 => { 3 },
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

#[derive(Debug)]
pub enum Token {
    LetExpr(String, String),
    Literal(String),
    Text(Option<Emphasis>, String),
    Comment(String),
    Markdown(MarkdownTag),
    Newline,
}

#[derive(Debug)]
pub enum Emphasis {
    Bold,
    Italic,
    Strikethrough,
}

#[derive(Debug)]
pub struct Tokenizer {
    src: String,
    pos: usize,
    max: usize,
    ctx: DebugContext,
}

impl Tokenizer {

    pub fn new(src: String, ctx: DebugContext) -> Self {
        let max = src.chars().count();
        Tokenizer {
            src,
            pos: 0,
            max,
            ctx,
        }
    }

    fn panic(&self, msg: &str) -> ! {
        self.ctx.panic(msg);
    }

    fn char(&mut self) -> char {
        self.src.chars().nth(self.pos).expect("No more characters")
    }

    fn peek(&mut self) -> char {
        self.src.chars().nth(self.pos + 1).unwrap_or('\0')
    }

    // peeks the char after the next char
    fn peek_n(&mut self, n: usize) -> char {
        self.src.chars().nth(self.pos + n).unwrap_or('\0')
    }

    fn advance_char(&mut self) {
        if self.pos >= self.max {
            return;
        }
        self.pos += 1;
    }

    fn must_consume(&mut self, c: char) {
        let curr = self.char();
        // [ERROR]
        if curr != c {
            // ! how to handle errors
            self.ctx.push_new_error(PrettyErrContext::new(
                PrettyErrKind::ExpectedButNotFound,
                (self.pos, self.pos + 1),
                vec![c.to_string(), curr.to_string()]
            ));
            self.panic(&format!("Expected '{}' but found '{}'", c, curr));
        }
        self.advance_char();
    }

    fn consume_whitespace(&mut self) {
        while self.char().is_whitespace() {
            self.pos += 1;
        }
    }

    fn consume_until_not(&mut self, c: char) -> &str {
        let start= self.pos;
        while self.char() == c {
            self.pos += 1;
        }
        &self.src[start..self.pos]
    }

    fn consume_till(&mut self, c: char) -> &str {
        let start = self.pos;
        while self.char() != c {
            self.pos += 1;
        }
        &self.src[start..self.pos]
    }

    fn consume_line(&mut self) -> &str {
        self.consume_till('\n')
    }

    pub fn next_line(&mut self) -> Option<Vec<Token>> {
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
        if self.pos >= self.max || self.char() == '\n' {
            return None;
        }

        let curr_tok = self.char();

        // consume comments
        if self.char() == '/' && self.peek() == '/' {
            self.advance_char();
            self.advance_char();
            let comment = self.consume_line().trim();
            return Some(Token::Comment(comment.to_string()));
        // literals
        } else if self.char() == '"' {
            self.advance_char();
            let literal = self.consume_till('"').to_string();
            self.must_consume('"');

            return Some(Token::Literal(literal));
        // let statements
        } else if self.char() == 'l' && self.peek() == 'e' && self.peek_n(2) == 't' {
            self.consume_whitespace();

            let var = self.consume_till('=').trim().to_string();
            // [ERROR] 
            // TODO: check if variable name is valid
            self.must_consume('=');
            self.consume_whitespace();

            let val = self.consume_till(';').trim().to_string();

            return Some(Token::LetExpr(var, val));
        // headers
        } else if self.char() == '#' {
            let hash_count = self.consume_until_not('#').len();

            let heading = self.consume_line().trim();
            let header_kind: HeaderKind = hash_count.into();

            return Some(
                Token::Markdown(
                    MarkdownTag::Header(header_kind, heading.to_string())
                )
            );
        // blockquote
        } else if curr_tok == '>' {
            self.advance_char();
            let blockquote = self.consume_line().trim();
            return Some(
                Token::Markdown(
                    MarkdownTag::Blockquote(blockquote.to_string())
                )
            );
        // bullets or checkboxes 
        } else if curr_tok == '-' {
            self.advance_char();
            self.consume_whitespace();

            let is_bullet = self.char() != '[';
            if is_bullet {
                let bullet = self.consume_line().trim();
                return Some(
                    Token::Markdown(
                        MarkdownTag::BulletPoint(bullet.to_string())
                    )
                );
            }

            self.advance_char();
            let is_checked = self.char() == 'x';

            self.advance_char();
            self.must_consume(']');
            self.consume_whitespace();

            let checkbox = self.consume_line().trim();
            return Some(
                Token::Markdown(
                    MarkdownTag::Checkbox(is_checked, checkbox.to_string())
                )
            );
        // line separator
        } else if curr_tok == '=' {

            self.consume_until_not('=');
            if self.consume_line().trim().len() > 0 {
                self.panic(&format!("Line separator should contain only '=' characters"));
            }

            // should contain only line separator
            // [ERROR]
            // if self.consume_line().trim().len() > 0 {
            //     self.panic();
            // }

            return Some(
                Token::Markdown(
                    MarkdownTag::LineSeparator
                )
            );
        // consume links
        } else if (curr_tok == '!' && self.peek() == '[') || curr_tok == '[' {
            let is_image = curr_tok == '!';
            if is_image {
                self.advance_char();
            }
            self.must_consume('[');
            let text = self.consume_till(']').to_string();
            self.must_consume(']');
            self.must_consume('(');
            let link = self.consume_till(')').to_string();
            self.must_consume(')');

            return Some(Token::Markdown(
                MarkdownTag::Link(
                    if is_image { LinkKind::Image } else { LinkKind::Hyperlink },
                    text,
                    link,
                )
            ));
        // code blocks
        } else if curr_tok == '`' {
            // check if inline code block or code block
            let code: String;
            if self.peek() == '`' {
                self.must_consume('`');
                self.must_consume('`');
                self.must_consume('`');

                self.consume_whitespace();
                code = self.consume_till('`').to_string();

                self.must_consume('`');
                self.must_consume('`');
                self.must_consume('`');
            } else {
                self.must_consume('`');
                code = self.consume_till('`').trim().to_string();
                self.must_consume('`');
            }

            return Some(Token::Markdown(
                MarkdownTag::CodeBlock(code)
            ));
        // bold
        } else if self.char() == '*' {
            if self.peek() == '*' {
                self.advance_char();
                self.advance_char();
                let text = self.consume_till('*').to_string();
                self.must_consume('*');
                self.must_consume('*');

                return Some(Token::Text(Some(Emphasis::Bold), text));
            } else {
                self.advance_char();
                let text = self.consume_till('*').to_string();
                self.must_consume('*');

                return Some(Token::Text(Some(Emphasis::Italic), text));
            }
        // strikethrough
        } else if self.char() == '~' {
            self.advance_char();
            let text = self.consume_till('~').to_string();
            self.must_consume('~');

            return Some(Token::Text(Some(Emphasis::Strikethrough), text));
        // text
        } else {
            let text = curr_tok.to_string();
            self.advance_char();
            return Some(Token::Text(None, text));
        }
    }

} 

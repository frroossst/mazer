use rayon::prelude::*;
use unicode_segmentation::UnicodeSegmentation;

use crate::pretty_err::{ErrCtx, ErrorKind, Span};

#[derive(Debug, Clone)]
pub enum MarkdownTag {
    Header(HeaderKind, String),
    LineSeparator,
    Checkbox(bool, String),
    BulletPoint(String),
    Blockquote(String),
    CodeBlock(String),
    Link(LinkKind, String, String),
    Table(Vec<String>, Vec<Vec<String>>),
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
    H3,
}

impl From<HeaderKind> for usize {
    fn from(value: HeaderKind) -> Self {
        match value {
            HeaderKind::H1 => 1,
            HeaderKind::H2 => 2,
            HeaderKind::H3 => 3,
        }
    }
}

impl From<usize> for HeaderKind {
    fn from(val: usize) -> Self {
        match val {
            1 => HeaderKind::H1,
            2 => HeaderKind::H2,
            3 => HeaderKind::H3,
            _ => HeaderKind::H1,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Token {
    LetStmt(String, String),
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

impl From<FnKind> for String {
    fn from(value: FnKind) -> Self {
        match value {
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
pub struct Lexer {
    src: Vec<String>,
    pos: usize,
    line: usize,
    max: usize,
    prv: Option<String>,
    ctx: ErrCtx,
}

impl Lexer {
    pub fn new(src: String, ctx: ErrCtx) -> Self {
        let mut uni_vec = UnicodeSegmentation::graphemes(src.as_str(), true)
            .collect::<Vec<&str>>()
            .par_iter()
            .map(|&x| x.to_string())
            .collect::<Vec<String>>();

        uni_vec.push("\n".to_string());
        // push invisible characters to the end of the vector
        // this is to ensure that the lexer does not run out of bounds
        let zero_width_character = vec![0xE2, 0x80, 0x8B];
        let invisible_char = String::from_utf8(zero_width_character).expect("Invalid UTF-8");

        let invisible_char = vec![invisible_char; 69];

        uni_vec.extend(invisible_char);

        let max = uni_vec.len();
        Lexer {
            src: uni_vec,
            pos: 0,
            line: 0,
            max,
            prv: None,
            ctx,
        }
    }

    fn create_error(&mut self, err: ErrorKind, span: Option<Span>) {
        self.ctx = self.ctx.clone().with_src(self.src.join(""));
        self.ctx = self.ctx.clone().with_error_kind(err.clone());

        if let Some(span_val) = span {
            self.ctx = self.ctx.clone().with_location(span_val.start, span_val.end);
        }

    }

    fn char(&mut self) -> Result<String, ErrCtx> {
        if self.pos >= self.max {
            // [ERROR]
            let e = ErrorKind::AbruptAdieu(format!(
                "Reached the end of file looking for position {}",
                self.pos
            ));
            self.create_error(e, Span { start: self.pos, end: self.pos + 1 }.into());
            return Err(self.ctx.clone());
        }
        Ok(self.src[self.pos].clone())
    }

    fn peek(&mut self) -> Result<String, ErrCtx> {
        if self.pos >= self.max {
            // [ERROR]
            let e = ErrorKind::AbruptAdieu(format!(
                "Reached the end of file looking for position {}",
                self.pos
            ));
            self.create_error(e, Span { start: self.pos + 1, end: self.pos + 2 }.into());
            Err(self.ctx.clone())
        } else {
            Ok(self.src[self.pos + 1].clone())
        }
    }

    // peeks the char after the next char
    fn peek_n(&mut self, n: usize) -> Result<String, ErrCtx> {
        if self.pos >= self.max {
            // [ERROR]
            let e = ErrorKind::AbruptAdieu(format!(
                "Reached the end of file looking for position {}",
                self.pos
            ));
            self.create_error(e, Span { start: self.pos + n, end: self.pos + n + 1 }.into());
            Err(self.ctx.clone())
        } else {
            Ok(self.src[self.pos + n].clone())
        }
    }

    fn advance_char(&mut self) -> Result<(), ErrCtx> {
        if self.pos >= self.max {
            let e = ErrorKind::AbruptAdieu(format!(
                "Reached the end of file looking for position {}",
                self.pos
            ));
            self.create_error(e, Span { start: self.pos, end: self.max }.into());
            return Err(self.ctx.clone());
        }
        self.pos += 1;
        Ok(())
    }

    fn must_consume(&mut self, c: &str) -> Result<(), ErrCtx> {
        let curr = self.char()?;
        // [ERROR]
        if curr != c {
            let e = ErrorKind::BrokenExpectations(format!("Expected '{}' but found '{}'", c, curr));
            self.create_error(e, Span { start: self.pos, end: self.pos + 1 }.into());
            return Err(self.ctx.clone());
        }
        self.advance_char()?;
        Ok(())
    }

    fn consume_whitespace(&mut self) -> Result<(), ErrCtx> {
        // keep moving forward if current string is made up of
        // whitespaces
        while self.char()?.trim().is_empty() {
            self.advance_char()?;
        }

        Ok(())
    }

    fn consume_until_not(&mut self, c: &str) -> Result<String, ErrCtx> {
        let start = self.pos;
        while self.char()? == c {
            self.pos += 1;
        }

        Ok(self.src[start..self.pos].join(""))
    }

    fn consume_till(&mut self, c: &str) -> Result<String, ErrCtx> {
        let start = self.pos;
        while self.char()? != c {
            self.pos += 1;
        }
        Ok(self.src[start..self.pos].join(""))
    }

    fn consume_line(&mut self) -> Result<String, ErrCtx> {
        self.consume_till("\n")
    }

    fn consume_nested_parenthesis(&mut self) -> Result<String, ErrCtx> {
        // iterate over source from current position
        // keep adding when ( is encountered
        // and decreasing when ) is encountered
        // if underflow then less opening
        // if overflow or reaches end of file then
        let mut store = String::from(self.char()?);
        let mut count = 1;

        let start_span = self.pos;

        while count > 0 {
            self.advance_char()?;
            if self.pos >= self.max {
                // [ERROR]
                let e = ErrorKind::LonelyParenthesis("Unmatched parenthesis".to_string());
                self.create_error(e, Span { start: start_span, end: self.pos }.into());
                return Err(self.ctx.clone());
            }

            let curr = self.char()?;
            store.push_str(&curr);

            if curr == "(" {
                count += 1;
            } else if curr == ")" {
                count -= 1;
            }
        }

        // check if balanced
        if count != 0 {
            // [ERROR]
            let e = ErrorKind::LonelyParenthesis("Unmatched parenthesis".to_string());
            self.create_error(e, Span { start: start_span, end: self.pos }.into());
            return Err(self.ctx.clone());
        }

        Ok(store)
    }

    fn consume_until_uneven_paren(&mut self) -> Result<String, ErrCtx> {
        // Consume until we have more closing parentheses than opening ones
        // This creates a "lispy" parsing behavior where extra closing parens
        // signal the end of the let statement
        let mut store = String::new();
        let mut paren_count = 0;

        while self.pos < self.max && self.char()? != "\n" {
            let curr = self.char()?;
            
            if curr == "(" {
                paren_count += 1;
                store.push_str(&curr);
                self.advance_char()?;
            } else if curr == ")" {
                paren_count -= 1;
                store.push_str(&curr);
                self.advance_char()?;
                
                // If we have more closing than opening parens, we've hit the end
                if paren_count < 0 {
                    break;
                }
            } else {
                store.push_str(&curr);
                self.advance_char()?;
            }
        }

        Ok(store.trim().to_string())
    }

    pub fn next_line(&mut self) -> Result<Option<Vec<Token>>, ErrCtx> {
        self.line += 1;
        if self.pos >= self.max {
            return Ok(None);
        }

        let mut tokens: Vec<Token> = Vec::new();
        while let Some(tok) = self.next_token()? {
            tokens.push(tok);
        }
        self.advance_char()?;

        if tokens.is_empty() {
            return Ok(Some(vec![Token::Newline]));
        }

        Ok(Some(tokens))
    }

    fn next_token(&mut self) -> Result<Option<Token>, ErrCtx> {
        if self.pos >= self.max || self.char()? == "\n" {
            if self.char()? == "\n" {
                self.prv = Some("\n".to_string());
            }
            return Ok(None);
        }

        let curr_tok = self.char()?;
        self.prv = None;

        // the tokenizer new adds these to prevent out of bounds
        // for valid source input
        if curr_tok == "\u{200b}" {
            // end of file reached
            return Ok(None);
        }

        // consume comments
        if curr_tok == "/" && self.peek()? == "/" {
            self.advance_char()?;
            self.advance_char()?;
            let comment = self.consume_line()?;
            let comment = comment.trim();
            Ok(Some(Token::Comment(comment.to_string())))
        // literals
        } else if curr_tok == "\"" {
            self.advance_char()?;
            let literal = self.consume_till("\"")?.to_string();
            self.must_consume("\"")?;

            return Ok(Some(Token::Literal(literal)));
        // let statements
        } else if curr_tok == "l" && self.peek()? == "e" && self.peek_n(2)? == "t" {
            let let_start_span = self.pos;
            self.advance_char()?;
            self.advance_char()?;
            self.advance_char()?;

            /*
            if self.char()? != " " {
                // [ERROR]
                let e = ErrorKind::GrammarGoblin(
                    "Let statement should be followed by a space".to_string(),
                );
                self.create_error(e, Span { start: self.pos, end: self.pos + 1 }.into());
                return Err(self.ctx.clone());
            } */

            let before_let_keyword_pos = self.pos;
            self.consume_whitespace()?;
            let did_consume_let_keyword = (self.pos - before_let_keyword_pos) > 0;

            // check if it conforms to the following pattern
            // let <var> <space> = <value>;
            // if not then it is a normal word 
            // if did_consume_let_keyword is false AND there is no equal to after variable name
            // then it is a normal word
            let curr = self.pos;
            let l = self.consume_line()?;
            // if l has no equal to sign then it is a normal word
            // NOTE: This is very scuffed and should be improved
            let has_equal = l.contains("=");
            if !did_consume_let_keyword && !has_equal {
                return Ok(Some(Token::Text(None, l)));
            } else {
                self.pos = curr;
            }

            let var = self.consume_till("=")?.trim().to_string();
            // [ERROR]
            if var.is_empty() {
                let e = ErrorKind::NamelessNomad("Variable name cannot be empty".to_string());
                self.create_error(e, Span { start: let_start_span, end: self.pos }.into());
                return Err(self.ctx.clone());
            }

            self.must_consume("=")?;
            let mut val = self.consume_until_uneven_paren()?.trim().to_string();
            
            // Remove any trailing parentheses that caused us to stop parsing
            while val.ends_with(')') {
                val.pop();
            }

            // sometimes things like let foo = (+ 1 2) \n\n let bar = (0); are valid; 
            // must prevent this by checking that value has only one let binding
            // if there are multiple let bindings then it is invalid

            val.contains("let")
                .then(|| {
                    let e = ErrorKind::GrammarGoblin(
                        "Let statement cannot be nested inside another let statement".to_string(),
                    );

                    let second_let_span = val
                        .find("let")
                        .unwrap_or(val.len())
                        + val.find("=")
                        .unwrap_or(val.len());

                    self.create_error(e, Span {
                        start: let_start_span,
                        end: second_let_span,
                    }.into());
                    Err(self.ctx.clone())
                })
                .unwrap_or(Ok(()))?;

            return Ok(Some(Token::LetStmt(var, val)));
        /*
        // inline fmt calls 
        } else if curr_tok == "$" && self.peek()? == "(" {
            self.advance_char()?;
            self.advance_char()?;
            self.must_consume(")")?;
            let fmt = self.consume_till(")")?.to_string();

            return Ok(Some(Token::Fn(FnKind::Fmt, fmt)));
         */
        // fmt calls
        } else if curr_tok == "f" && self.peek()? == "m" && self.peek_n(2)? == "t" {
            self.advance_char()?;
            self.advance_char()?;
            self.advance_char()?;
            self.must_consume("(")?;

            let mut fmt = String::new();
            if self.char()? != ")" {
                // the body expression may have parenthesis in it, so need to maintain a stack and
                // consume until the stack is empty
                fmt = self.consume_nested_parenthesis()?.trim().to_string();
                // remove the last character
                fmt.pop();
            }

            self.must_consume(")")?;

            return Ok(Some(Token::Fn(FnKind::Fmt, fmt)));
        // eval calls
        } else if curr_tok == "e"
            && self.peek()? == "v"
            && self.peek_n(2)? == "a"
            && self.peek_n(3)? == "l"
        {
            self.advance_char()?;
            self.advance_char()?;
            self.advance_char()?;
            self.advance_char()?;
            self.must_consume("(")?;

            let mut eval = String::new();
            if self.char()? != ")" {
                eval = self.consume_nested_parenthesis()?.trim().to_string();
                // remove the last character
                eval.pop();
            }

            self.must_consume(")")?;

            return Ok(Some(Token::Fn(FnKind::Eval, eval)));
        // headers
        } else if curr_tok == "#" {
            let hash_count = self.consume_until_not("#")?.len();

            let heading = self.consume_line()?;
            let heading = heading.trim();

            let header_kind: HeaderKind = hash_count.into();

            return Ok(Some(Token::Markdown(MarkdownTag::Header(
                header_kind,
                heading.to_string(),
            ))));
        // blockquote
        } else if curr_tok == ">" {
            self.advance_char()?;
            // only a blockquote if previously it was a newline
            if self.prv.is_some() && self.prv.clone().unwrap() == "\n" {
                let blockquote = self.consume_line()?;
                let blockquote = blockquote.trim();
                return Ok(Some(Token::Markdown(MarkdownTag::Blockquote(
                    blockquote.to_string(),
                ))));
            // this is to ensure an arrow like this, "->" can be made in text
            } else {
                return Ok(Some(Token::Text(None, String::from(">"))));
            }

        // bullets or checkboxes
        } else if curr_tok == "-" {
            self.advance_char()?;
            self.consume_whitespace()?;

            let mut is_bullet = self.char()? != "[";

            // only a bullet if the next character is an ascii alphabet number
            let is_next_alnum = self.peek()?.chars().next().unwrap().is_ascii_alphanumeric();
            is_bullet = is_bullet && is_next_alnum;

            if is_bullet {
                let bullet = self.consume_line()?;
                let bullet = bullet.trim();
                return Ok(Some(Token::Markdown(MarkdownTag::BulletPoint(
                    bullet.to_string(),
                ))));
            }

            let is_checkbox = self.char()? == "[";
            if is_checkbox {
                self.advance_char()?;
                let is_checked = self.char()? == "x";

                self.advance_char()?;
                self.must_consume("]")?;
                self.consume_whitespace()?;

                let checkbox = self.consume_line()?;
                let checkbox = checkbox.trim();
                return Ok(Some(Token::Markdown(MarkdownTag::Checkbox(
                    is_checked,
                    checkbox.to_string(),
                ))));
            }

            return Ok(Some(Token::Text(None, curr_tok.to_string())));

        // line separator
        } else if curr_tok == "=" && self.peek()? == "=" && self.peek_n(2)? == "=" {
            let prev = self.pos;
            self.consume_until_not("=")?;
            let now = self.pos;
            if (now - prev == 3) && !self.consume_line()?.trim().is_empty() {
                let e = ErrorKind::GrammarGoblin(
                    "Line separator should contain only '=' characters".to_string(),
                );
                self.create_error(e, Span { start: prev, end: now }.into());
                return Err(self.ctx.clone());
            }

            return Ok(Some(Token::Markdown(MarkdownTag::LineSeparator)));
        // consume links
        } else if (curr_tok == "!" && self.peek()? == "[") || curr_tok == "[" {
            let is_image = curr_tok == "!";
            if is_image {
                self.advance_char()?;
            }
            self.must_consume("[")?;
            let text = self.consume_till("]")?.to_string();
            self.must_consume("]")?;
            self.must_consume("(")?;
            let link = self.consume_till(")")?.to_string();
            self.must_consume(")")?;

            return Ok(Some(Token::Markdown(MarkdownTag::Link(
                if is_image {
                    LinkKind::Image
                } else {
                    LinkKind::Hyperlink
                },
                text,
                link,
            ))));
        // code blocks
        } else if curr_tok == "`" {
            // check if inline code block or code block
            let code: String;
            if self.peek()? == "`" {
                self.must_consume("`")?;
                self.must_consume("`")?;
                self.must_consume("`")?;

                self.consume_whitespace()?;
                code = self.consume_till("`")?.to_string();

                self.must_consume("`")?;
                self.must_consume("`")?;
                self.must_consume("`")?;
            } else {
                self.must_consume("`")?;
                code = self.consume_till("`")?.trim().to_string();
                self.must_consume("`")?;
            }

            return Ok(Some(Token::Markdown(MarkdownTag::CodeBlock(code))));
        // bold
        } else if curr_tok == "*" {
            if self.peek()? == "*" {
                self.advance_char()?;
                self.advance_char()?;
                let text = self.consume_till("*")?.to_string();
                self.must_consume("*")?;
                self.must_consume("*")?;

                return Ok(Some(Token::Text(Some(Emphasis::Bold), text)));
            } else {
                self.advance_char()?;
                let text = self.consume_till("*")?.to_string();
                self.must_consume("*")?;

                return Ok(Some(Token::Text(Some(Emphasis::Italic), text)));
            }
        // strikethrough
        } else if curr_tok == "~" {
            self.advance_char()?;
            let text = self.consume_till("~")?.to_string();
            self.must_consume("~")?;

            return Ok(Some(Token::Text(Some(Emphasis::Strikethrough), text)));
        // tables
        } else if curr_tok == "|" {
            self.advance_char()?;
            // tables be like
            // | col1 | col2 | col3 |
            // ---
            // | val1 | val2 | val3 |
            // | val4 | val5 | val6 |

            let table_header: Vec<String> = self.consume_line()?.split("|")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            let column_count = table_header.len();

            // consume the line separator
            self.consume_whitespace()?;
            self.must_consume("-")?;
            self.must_consume("-")?;
            self.must_consume("-")?;
            self.consume_whitespace()?;

            // keep consuming lines as long as they begin with |
            let mut table_body_unparsed = String::new();
            while self.char()? == "|" {
                self.consume_whitespace()?;

                let row = self.consume_line()?;
                // remove the ending |
                let row = row.trim_end_matches("|").to_string();
                table_body_unparsed.push_str(&row);

                self.consume_whitespace()?;
            }

            let table_body = table_body_unparsed
                .split("|")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>();


            let table_body_grouped: Vec<Vec<String>> = table_body.chunks(column_count).map(|chunk| chunk.to_vec()).collect();

            return Ok(Some(Token::Markdown(MarkdownTag::Table(
                table_header,
                table_body_grouped,
            ))));

        // text
        } else {
            let text = curr_tok.to_string();
            self.advance_char()?;
            return Ok(Some(Token::Text(None, text)));
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

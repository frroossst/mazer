use crate::{lexer::Token, tokenizer::MarkdownTag};


#[derive(Debug)]
pub struct Parser {
    src: Vec<Token>,
    pos: usize,
}

impl Parser {

    pub fn new(src: Vec<Token>) -> Self {
        Parser { 
            src,
            pos: 0,
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
            unimplemented!();
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
            unimplemented!();
        } else {
            None
        }
    }

}

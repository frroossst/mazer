use std::io::Write;

use maple_core::{document::Document, pretty_err::DebugContext, tokenizer::{Token, Tokenizer}};


#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
#[derive(Debug, Clone)]
struct Args {
    /// Maple file to run
    file: String,
}

fn main() {

    let args = <Args as clap::Parser>::parse();

    let file = std::fs::File::open(args.file.clone()).unwrap();
    let mut reader = std::io::BufReader::new(file);
    let mut content = String::new();
    std::io::Read::read_to_string(&mut reader, &mut content).unwrap();

    // get name of the file from the path to act as the title of HTML page
    let file_name_title = args.file.split("/").last().unwrap().split(".").next().unwrap();

    let document = to_document2(file_name_title, content);
    let out = document.output();

    // write document output to output.html 
    let mut file = std::fs::File::create("output.html").unwrap();
    write!(&mut file, "{}", out).unwrap();
}

fn to_document2(file_title: &str, content: String) -> Document {
    let debug_info = DebugContext::new(file_title);
    let mut t: Tokenizer = Tokenizer::new(content, debug_info);

    let mut all_tokens: Vec<Token> = Vec::with_capacity(512);
    while let Some(line) = t.next_line() {
        dbg!(&line);
        all_tokens.extend(line);
    }

    // the vector might have repeated Text tokens that need to be merged
    // into one. 
    // Vec[Token::Text('a'), Token::Text('b'), Token::Text('c'), Token::Text('d')]
    // should be translated to
    // Vec[Token::Text('abcd')]
    // Token::Text(Option<Emphasis>, Sting), only text pieces with None emphasis should combine


    dbg!(all_tokens.len());

    let mut document: Document = Document::new(file_title);

    document
}

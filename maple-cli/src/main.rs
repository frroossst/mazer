use std::io::Write;

use maple::{document::Document, pretty_err::DebugContext, tokenizer::Tokenizer};


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


    while let Some(line) = t.next_line() {
        dbg!(line);
    }

    let mut document: Document = Document::new(file_title);

    document
}


#[allow(dead_code)]
fn to_document(file_title: &str, _content: String) -> Document {
    // let mut p: Parser = Parser::new(content);
    let document: Document = Document::new(file_title);

    /*
    while let Some(line) = p.next_tagged() {
        match line {
            LineTag::Markdown(tag) => {
                document.add_markdown(tag);
            },  
            LineTag::Let(line) => {
                document.add_let(line);
            },
            LineTag::Text(line) => {
                document.add_text(line);
            },
        }
    }
    p.resolve_ctx();
    */


    document
}


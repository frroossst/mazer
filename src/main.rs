use std::io::Write;

use maple::{document::Document, parser::{LineTag, Parser}};



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

    let mut p: Parser = Parser::new(content.clone());
    let mut document: Document = Document::new(file_name_title);

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

    let out = document.output();

    // write document output to output.html 
    let mut file = std::fs::File::create("output.html").unwrap();
    write!(&mut file, "{}", out).unwrap();

    dbg!(p);
}


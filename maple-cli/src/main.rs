use std::{convert::Infallible, io::Write};

use maple_core::{document::Document, pretty_err::DebugContext, tokenizer::{Token, Tokenizer}};
use warp::Filter;


#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
#[derive(Debug, Clone)]
struct Args {
    /// Maple file to run
    file: String,
    /// Serve the file on a local server
    #[clap(short, long)]
    serve: bool,
}



#[tokio::main]
async fn main() {

    let args = <Args as clap::Parser>::parse();

    let file = std::fs::File::open(args.file.clone()).unwrap();
    let mut reader = std::io::BufReader::new(file);
    let mut content = String::new();
    std::io::Read::read_to_string(&mut reader, &mut content).unwrap();

    // get name of the file from the path to act as the title of HTML page
    let file_name_title = args.file.split("/").last().unwrap().split(".").next().unwrap();

    if args.serve {
        let index_route = warp::path::end().and(warp::fs::file("output.html"));
        let serve_route = warp::path("serve")
                                .and(warp::get())
                                .and_then(serve_route);
        let version_route = warp::path("version")
                                .and(warp::get())
                                .and_then(version_route);

        let routes = index_route.or(serve_route).or(version_route);

        let port: u16 = 58050;

        println!("Serving on http://127.0.0.1:{}", port);
        println!("Press Ctrl+C to stop the server\n");

        warp::serve(routes).run(([127, 0, 0, 1], port)).await;
    }

    let document = to_document2(file_name_title, content);
    let out = document.output();

    // write document output to output.html 
    let mut file = std::fs::File::create("output.html").unwrap();
    write!(&mut file, "{}", out).unwrap();
}

async fn version_route() -> Result<impl warp::Reply, Infallible> {
    // get version from toml file
    let version = env!("CARGO_PKG_VERSION");
    Ok(warp::reply::html(version))
}

async fn serve_route() -> Result<impl warp::Reply, Infallible> {
    Ok(warp::reply::html("Hello, World!"))
}

fn to_document2(file_title: &str, content: String) -> Document {
    let debug_info = DebugContext::new(file_title);
    let mut t: Tokenizer = Tokenizer::new(content, debug_info);

    let mut tokens: Vec<Token> = Vec::with_capacity(512);
    while let Some(line) = t.next_line() {
        dbg!(&line);
        tokens.extend(line);
    }

    let tokens = Tokenizer::compact(tokens);

    // the vector might have repeated Text tokens that need to be merged
    // into one. 
    // Vec[Token::Text('a'), Token::Text('b'), Token::Text('c'), Token::Text('d')]
    // should be translated to
    // Vec[Token::Text('abcd')]
    // Token::Text(Option<Emphasis>, Sting), only text pieces with None emphasis should combine


    dbg!(tokens.len());

    let mut document: Document = Document::new(file_title);

    document
}

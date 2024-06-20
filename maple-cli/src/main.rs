use std::{convert::Infallible, sync::{Arc, Mutex}};

use maple_core::{document::Document, pretty_err::DebugContext, tokenizer::{Token, Tokenizer}};

use crypto_hash::{Algorithm, hex_digest};
use warp::{reject::Rejection, reply::Reply, Filter};


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

struct State {
    path: String,
    title: String,
    hash: Option<String>,
}

impl State {
    fn new(path: String, title: String, hash: Option<String>) -> Self {
        Self { path, title, hash }
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn path(&self) -> String {
        self.path.clone()
    }

    fn hash(&self) -> Option<String> {
        self.hash.clone()
    }

    fn set_hash(&mut self, hash: String) {
        self.hash = Some(hash);
    }
}

#[tokio::main]
async fn main() {

    let args = <Args as clap::Parser>::parse();


    // get name of the file from the path to act as the title of HTML page
    let file_name_title = args.file.split("/").last().unwrap().split(".").next().unwrap();

    let state = State::new(args.file.clone(), file_name_title.to_string(), None);
    let state = Arc::new(Mutex::new(state));

    if args.serve {
        let index_route = warp::path::end().and(warp::fs::file("maple-cli/index.html"));
        let serve_route = warp::path("serve")
                                .and(warp::get())
                                .and_then(move || serve_route(state.clone()));

        let version_route = warp::path("version")
                                .and(warp::get())
                                .and_then(version_route);

        let routes = index_route.or(serve_route).or(version_route);

        let port: u16 = 58050;
        println!("Serving on http://127.0.0.1:{}", port);
        println!("Press Ctrl+C to stop the server\n");
        warp::serve(routes).run(([127, 0, 0, 1], port)).await;
    }


    let doc = Document::new(file_name_title);
    let out = doc.output();
    // write to output.html
    std::fs::write(format!("{}.html", doc.title()), &out).expect("Failed to write to output.html");

}

async fn version_route() -> Result<impl warp::Reply, Infallible> {
    // get version from toml file
    let version = env!("CARGO_PKG_VERSION");
    Ok(warp::reply::html(version))
}

async fn serve_route(state: Arc<Mutex<State>>) -> Result<Box<dyn Reply>, Rejection> {

    // check if a hash exists
    let mut state = state.lock().expect("Failed to lock state");
    let hash = state.hash();

    let path = state.path();
    let title = state.title();

    let file_read = read_file(&path, hash);
    if file_read.is_none() {
        // I want to return something so that if the client
        // has data if doesn't go blank or change the data
        // insert code
        return Ok(
            Box::new(
                warp::reply::with_status(
                    "", 
                    warp::http::StatusCode::NOT_MODIFIED)
            )
        );
    }

    let (new_content, new_hash) = file_read.unwrap();
    state.set_hash(new_hash);

    let document = to_document2(&title, new_content);
    let out = document.output();

    Ok(
        Box::new(
            warp::reply::html(out)
        )
    )
}

/// Read a file and return its content and hash
/// Only returns the content if the hash has changed
fn read_file(file_path: &str, hash: Option<String>) -> Option<(String, String)> {
    let fobj = std::fs::File::open(file_path).expect("Failed to open file");
    let mut reader = std::io::BufReader::new(fobj);
    let mut content = String::new();
    std::io::Read::read_to_string(&mut reader, &mut content).unwrap();

    let new_hash = hex_digest(Algorithm::SHA256, content.as_bytes());

    if hash.is_some() && hash.unwrap() == new_hash {
        None
    } else {
        Some((content, new_hash))
    }
}


fn to_document2(file_title: &str, content: String) -> Document {
    let debug_info = DebugContext::new(file_title);
    let mut t: Tokenizer = Tokenizer::new(content, debug_info);

    let mut tokens: Vec<Token> = Vec::with_capacity(512);
    while let Some(line) = t.next_line() {
        tokens.extend(line);
    }

    let tokens = Tokenizer::compact(tokens);
    let mut document: Document = Document::new(file_title);

    for t in tokens { 
        match t {
            Token::LetExpr(var, val) => {
                document.append_code(&format!("let {} = {}", var, val));
                document.append_newline();
            },
            Token::Fn(kind, expr) => {
                let kind_str: String = kind.into();
                document.append_wrapped_with_attr("span", "class=inline-code", &format!("{}({})", kind_str, expr));
            },
            Token::Literal(lit) => {
                document.append_text( None, &lit);
            },
            Token::Text(emp, txt) => {
                document.append_text(emp, &txt);
            },
            Token::Comment(_) => {
                // do nothing
            },
            Token::Markdown(tag) => {
                document.add_markdown(tag);
            },
            Token::Newline => {
                document.append_newline();
            },
        }
    }

    document
}

use std::{convert::Infallible, sync::{Arc, Mutex}};

use maple_core::{document::Document, pretty_err::DebugContext, tokenizer::{Token, Tokenizer}};
use maple_cli::state::State;

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
    /// Open the file in the default browser
    #[clap(short, long)]
    open: bool,
}



#[tokio::main]
async fn main() {

    let args = <Args as clap::Parser>::parse();

    // get name of the file from the path to act as the title of HTML page
    let file_name_title = args.file.split("/").last().unwrap().split(".").next().unwrap();

    let state = State::new(args.file.clone(), file_name_title.to_string());
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
        let link = format!("http://127.0.0.1:{}", port);
        println!("Serving on {}", link);
        println!("Press Ctrl+C to stop the server\n");

        if args.open {
            opener::open(link).expect("Failed to open the default browser");
        }

        warp::serve(routes).run(([127, 0, 0, 1], port)).await;
    }

    let content = read_file(&args.file);
    let doc= to_document(file_name_title, content);
    let out = doc.output();
    // create and write to file
    let mut file = std::fs::File::create(format!("{}.html", file_name_title)).expect("Failed to create file");
    std::io::Write::write_all(&mut file, out.as_bytes()).expect("Failed to write to file");
}

async fn version_route() -> Result<impl warp::Reply, Infallible> {
    // get version from toml file
    let version = env!("CARGO_PKG_VERSION");
    Ok(warp::reply::html(version))
}

async fn serve_route(state: Arc<Mutex<State>>) -> Result<Box<dyn Reply>, Rejection> {
    // check if a hash exists
    let mut state = state.lock().expect("Failed to lock state");

    let path = state.path();
    let title = state.title();

    let has_changed = state.has_file_changed();
    if !has_changed {
        Ok(
            Box::new(
                warp::reply::with_status(
                    "", 
                    warp::http::StatusCode::NOT_MODIFIED)
            )
        )
    } else {
        let content = read_file(&path);
        let document = to_document(&title, content);
        let out = document.output();

        Ok(
            Box::new(
                warp::reply::html(out)
            )
        )
    }


}

/// Read a file and return its content 
fn read_file(file_path: &str) -> String {
    let fobj = std::fs::File::open(file_path).expect("Failed to open file");
    let mut reader = std::io::BufReader::new(fobj);
    let mut content = String::new();
    std::io::Read::read_to_string(&mut reader, &mut content).unwrap();

    content
}


fn to_document(file_title: &str, content: String) -> Document {
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
                document.append_code(&format!("{} = {}", var, val));
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

use std::{convert::Infallible, sync::{Arc, Mutex}};

use mazer_core::{document::Document, interpreter::Interpreter, parser::{ASTNode, Parser}, pretty_err::DebugContext, tokenizer::{FnKind, Lexer, Token}};
use mazer_cli::state::State;

use warp::{reject::Rejection, reply::Reply, Filter};
use colored::*;


#[derive(clap::Parser)] #[command(version, about, long_about = None)]
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
    /// Dry-run, does not create html file as output
    #[clap(short, long)]
    dry_run: bool,
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
    let (doc, ctx)= to_document(file_name_title, content, &args.file.as_str());
    if ctx.is_some() {
        ctx.unwrap().display();
    } else {
        println!("{}", format!("[INFO] No errors, {} ", "OK".green().bold()));
    }
    let out = doc.output();
    
    if !args.dry_run {
        // create and write to file
        let mut file = std::fs::File::create(format!("{}.html", file_name_title)).expect("Failed to create file");
        std::io::Write::write_all(&mut file, out.as_bytes()).expect("Failed to write to file");
    }
}

async fn version_route() -> Result<impl warp::Reply, Infallible> {
    // get version from toml file
    let version = env!("CARGO_PKG_VERSION");
    Ok(warp::reply::html(version))
}

async fn serve_route(state: Arc<Mutex<State>>) -> Result<Box<dyn Reply>, Rejection> {
    let (path, title, has_changed) = {
        let mut state = state.lock().expect("Failed to lock state");
        (state.path().clone(), state.title().clone(), state.has_file_changed())
    };

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
        let (document, context) = to_document(&title, content, &path);
        if context.is_some() {
            context.unwrap().display();
        } else {
            println!("{}", format!("[INFO] No errors, {} ", "OK".green().bold()));
        }
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


fn to_document(file_title: &str, content: String, file_path: &str) -> (Document, Option<DebugContext>) {
    let mut t: Lexer = Lexer::new(content, DebugContext::new(file_path));

    let mut tokens: Vec<Token> = Vec::with_capacity(512);

    let mut ctx: Option<DebugContext> = None;
    loop {
        match t.next_line() {
            Ok(Some(l)) => {
                tokens.extend(l);
            },
            Ok(None) => break,
            Err(e) => {
                ctx = Some(e);
                break;
            }
        }
    }

    let tokens = Lexer::compact(tokens);

    // handle for the document that outputs HTML
    let mut document: Document = Document::new(file_title);

    // handle for the interpreter that emits MathML or values
    // we reset the debug context as we need the file_path but do not need other debug info, as 
    // we will be setting new interpreter specific and later parser specific debug info
    let mut interp: Interpreter = Interpreter::new(DebugContext::new(file_path));

    for t in tokens { 
        match t {
            Token::LetExpr(var, val) => {
                let stmt = format!("let {} = {}", &var, &val);
                document.append_code(&stmt);

                let node: Result<Vec<ASTNode>, DebugContext> = Parser::new(stmt, DebugContext::new(file_path)).parse();
                match node {
                    Ok(n) => {
                        interp.add_chunk(var, n);
                    },
                    // this path means there is a syntax error
                    Err(e) => {
                        ctx = Some(e);
                        break;
                    }
                }
                document.append_newline();
            },
            Token::Fn(kind, expr) => {
                let p_out = Parser::new(expr.clone(), DebugContext::new(file_path)).parse();
                let node = match p_out {
                    Ok(n) => { n },
                    // this path means there is a syntax error
                    Err(e) => {
                        ctx = Some(e);
                        break;
                    }
                };

                let symbol = "c7eb03ac0c02f209437c28381c4d656dca8b98fbf73a062c77cbdc7bb7de93";
                let symbol = symbol.to_string();

                interp.add_chunk(symbol.clone(), node);

                match kind {
                    FnKind::Eval => {
                        let eval = interp.eval(symbol);
                        document.append_text(None, &eval.to_string());
                    },
                    FnKind::Fmt => {
                        let markup = interp.fmt(symbol);
                        document.append_math_ml(&markup);
                    },
                }
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

    // type checking and syntax errors
    // compile the language to bytecode?

    // get doc replacements
    // let replacements = parser.get_replacements();
    // document.replace(Vec<(orig, new)>);

    (document, ctx)
}

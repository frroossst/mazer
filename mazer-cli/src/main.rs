use std::io::{self, Write};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};

use mazer_cli::state::State;
use mazer_core::parser::MathML;
use mazer_core::{
    document::Document,
    interpreter::Interpreter,
    parser::Parser,
    pretty_err::DebugContext,
    tokenizer::{FnKind, Lexer, Token},
};

use colored::*;
use warp::{reject::Rejection, reply::Reply, Filter};

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
#[derive(Debug, Clone)]
struct Args {
    /// Maple file to run
    file: Option<String>,
    /// Serve the file on a local server
    #[clap(short, long)]
    serve: bool,
    /// Open the file in the default browser
    #[clap(short, long)]
    open: bool,
    #[clap(short, long)]
    repl: bool,
    /// Dry-run, does not create html file as output
    #[clap(short, long)]
    dry_run: bool,
}

fn print_help_message() {
    eprintln!("Usage: mazer [OPTIONS] <file>");
    eprintln!();
    eprintln!("--serve      Serve the file on a local server");
    eprintln!("--open       Open the file in the default browser");
    eprintln!("--repl       Start the REPL");
    eprintln!("--dry-run    Dry-run, does not create html file as output");
    eprintln!();
}

fn prompt() -> String {
    print!(">>> ");
    let mut out = io::stdout().lock();
    out.flush().expect("unable to flush");

    let mut buffer = String::new();
    loop {
        let num_bytes = std::io::stdin()
            .read_line(&mut buffer)
            .expect("unable to read line!");
        if num_bytes == 0 {
            std::process::exit(0);
        }

        let prev_len = buffer.len();
        if (prev_len - buffer.trim_end_matches('\n').len()) >= 2 {
            break;
        } else if "q" == buffer.as_str().trim() {
            std::process::exit(0);
        }
    }
    buffer.trim().to_string()
}

#[tokio::main]
async fn main() {
    let args = <Args as clap::Parser>::parse();

    if args.repl {
        println!("welcome to the mazer REPL");
        println!("press Enter twice (i.e. blank line) to execute");
        println!("q to quit");
        println!();
        println!("wrap code in fmt() to output equivalent MathML");
        println!("wrap code in eval() to evaluate the expression");
        println!();

        let env_path = std::env::current_dir()
            .expect("cannot get current working directory")
            .to_str()
            .unwrap()
            .to_owned()
            + "<REPL>";

        let _interp: Interpreter = Interpreter::new(DebugContext::new(&env_path));

        loop {
            let src = prompt();

            let mut tokens: Vec<Token> = Vec::new();
            let mut t: Lexer = Lexer::new(src, DebugContext::new(&env_path));
            loop {
                match t.next_line() {
                    Ok(Some(l)) => {
                        tokens.extend(l);
                    }
                    Ok(None) => break,
                    Err(e) => {
                        eprintln!("{:?}", e);
                        break;
                    }
                }
            }
            let tokens = Lexer::compact(tokens);
            let tokens = tokens
                .into_iter()
                .filter(|t| match t.clone() {
                    Token::LetExpr(_var, _val) => true,
                    Token::Fn(_kind, _body) => true,
                    _ => {
                        eprintln!("[ERROR] repl can only process mazer tokens");
                        false
                    }
                })
                .collect::<Vec<Token>>();

            for t in tokens.iter() {
                match t {
                    Token::LetExpr(_var, _val) => {
                        unimplemented!("let expr");
                    }
                    Token::Fn(_kind, _expr) => {
                        unimplemented!("fn kind");
                    }
                    _ => {}
                }
            }
            eprintln!("evaluating....");
        }
    }

    let file;
    match args.file {
        Some(f) => {
            file = f;
        }
        None => {
            eprintln!("{}", "no file name given".bright_red().bold());
            print_help_message();
            std::process::exit(1);
        }
    }

    // get name of the file from the path to act as the title of HTML page
    let file_name_title = file.split("/").last().unwrap().split(".").next().unwrap();

    let state = State::new(file.clone(), file_name_title.to_string());
    let state = Arc::new(Mutex::new(state));

    if args.serve {
        let index_route = warp::path::end().and(warp::fs::file("mazer-cli/index.html"));
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

    let content = read_file(&file);
    let (doc, ctx) = to_document(file_name_title, content, &file.as_str());
    if ctx.is_some() {
        ctx.unwrap().display();
    } else {
        println!(
            "{}",
            format!("{} No errors, {} ", "[INFO]".yellow(), "OK".green().bold())
        );
    }
    let out = doc.output();

    if !args.dry_run {
        // create and write to file
        std::fs::create_dir_all("out").expect("Failed to create directory");
        let mut file = std::fs::File::create(format!("out/{}.html", file_name_title))
            .expect("Failed to create file");
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
        (
            state.path().clone(),
            state.title().clone(),
            state.has_file_changed(),
        )
    };
    if !has_changed {
        Ok(Box::new(warp::reply::with_status(
            "",
            warp::http::StatusCode::NOT_MODIFIED,
        )))
    } else {
        let content = read_file(&path);
        let (document, context) = to_document(&title, content, &path);
        if context.is_some() {
            context.unwrap().display();
        } else {
            println!(
                "{}",
                format!("{} No errors, {} ", "[INFO]".yellow(), "OK".green().bold())
            );
        }
        let out = document.output();

        Ok(Box::new(warp::reply::html(out)))
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

fn to_document(
    file_title: &str,
    content: String,
    file_path: &str,
) -> (Document, Option<DebugContext>) {
    let mut t: Lexer = Lexer::new(content, DebugContext::new(file_path));

    let mut tokens: Vec<Token> = Vec::with_capacity(512);

    let mut ctx: Option<DebugContext> = None;
    loop {
        match t.next_line() {
            Ok(Some(l)) => {
                tokens.extend(l);
            }
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
                document.append_code(&format!("let {} = {}", &var, &val));

                let mut p = Parser::new(val);
                let expr = p.parse();
                interp.set_chunk(var, expr);
            }
            Token::Fn(kind, expr) => match kind {
                FnKind::Eval => {
                    document.append_code(&format!("unable to evaluate = eval({})", &expr));
                }
                FnKind::Fmt => {
                    let mut p = Parser::new(expr);
                    let exprs = p.parse();

                    dbg!(&exprs);

                    let _mathml: MathML = exprs.into();

                    unimplemented!("fn kind fmt");
                }
            },
            Token::Literal(lit) => {
                document.append_text(None, &lit);
            }
            Token::Text(emp, txt) => {
                document.append_text(emp, &txt);
            }
            Token::Comment(_) => {
                // do nothing
            }
            Token::Markdown(tag) => {
                document.add_markdown(tag);
            }
            Token::Newline => {
                document.append_newline();
            }
        }
    }

    // type checking and syntax errors
    // compile the language to bytecode?

    // get doc replacements
    // let replacements = parser.get_replacements();
    // document.replace(Vec<(orig, new)>);

    (document, ctx)
}

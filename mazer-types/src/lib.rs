pub mod environment;

pub use environment::Environment;

#[derive(Debug, Clone, Hash)]
pub enum LispAST {
    Error(String),
    Symbol(String),
    Number(fastnum::D512),
    Bool(bool),
    String(String),
    List(Vec<LispAST>),
    Application { name: String, args: Vec<LispAST> },
    UserFunc { params: Vec<String>, body: Box<LispAST> },
    NativeFunc(fn(&[LispAST]) -> Result<LispAST, String>),
}

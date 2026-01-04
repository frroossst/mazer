#[derive(Debug, Clone, Hash)]
pub enum LispAST {
    Error(String),
    Symbol(String),
    Number(fastnum::D512),
    Bool(bool),
    List(Vec<LispAST>),
    Application { name: String, args: Vec<LispAST> },
    NativeFunc(fn(&[LispAST]) -> Result<LispAST, String>),
    UserFunc { params: Vec<String>, body: Box<LispAST> },
}

use fastnum::D512;


#[derive(Debug, Clone, Hash)]
pub enum LispAST {
    Symbol(String),
    Number(D512),
    Bool(bool),
    List(Vec<LispAST>),
    Application { name: String, args: Vec<LispAST> },
    NativeFunc(fn(&[LispAST]) -> Result<LispAST, String>),
}

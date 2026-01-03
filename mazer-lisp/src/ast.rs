use core::fmt;

use fastnum::D512;


#[derive(Debug, Clone, Hash)]
pub enum LispAST {
    Error(String),
    Symbol(String),
    Number(D512),
    Bool(bool),
    List(Vec<LispAST>),
    Application { name: String, args: Vec<LispAST> },
    NativeFunc(fn(&[LispAST]) -> Result<LispAST, String>),
}

impl From<&LispAST> for String {
    // TODO:
    fn from(ast: &LispAST) -> Self {
        match ast {
            LispAST::Error(s) => format!("Error({})", s),
            LispAST::Symbol(s) => s.clone(),
            LispAST::Number(n) => n.to_string(),
            LispAST::Bool(b) => b.to_string(),
            LispAST::List(lst) => {
                let elements: Vec<String> = lst.iter().map(|elem| String::from(elem)).collect();
                format!("({})", elements.join(" "))
            }
            LispAST::Application { name, args } => {
                let arg_strs: Vec<String> = args.iter().map(|arg| String::from(arg)).collect();
                format!("{}({})", name, arg_strs.join(" "))
            }
            LispAST::NativeFunc(_) => "<native-func>".to_string(),
        }
    }
} 


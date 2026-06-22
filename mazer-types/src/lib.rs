pub mod environment;
pub mod error;
pub mod implfuncs;

pub use environment::Environment;
pub use error::LispError;

#[derive(Debug, Clone, Hash)]
pub enum LispAST {
    Error(String),
    Symbol(String),
    Number(fastnum::D512),
    Bool(bool),
    String(String),
    List(Vec<LispAST>),
    Application {
        name: String,
        args: Vec<LispAST>,
    },
    UserFunc {
        params: Vec<String>,
        body: Box<LispAST>,
    },
    NativeFunc(fn(&[LispAST]) -> Result<LispAST, LispError>),
}

impl LispAST {
    /// The human-readable type name of this value, used in error messages.
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self {
            LispAST::Error(_) => "Error",
            LispAST::Symbol(_) => "Symbol",
            LispAST::Number(_) => "Number",
            LispAST::Bool(_) => "Bool",
            LispAST::String(_) => "String",
            LispAST::List(_) => "List",
            LispAST::Application { .. } => "Application",
            LispAST::UserFunc { .. } => "UserFunc",
            LispAST::NativeFunc(_) => "NativeFunc",
        }
    }
}

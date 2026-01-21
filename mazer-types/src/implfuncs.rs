use strum_macros::EnumIter;

#[derive(Debug, EnumIter)]
pub enum ShowFunc {
    MaybeFunc(String),

    Define,
    Defunc,
    Quote,
    String,

    Jux,

    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Frac,
    Sqrt,
    Root,

    Eq,
    Neq,
    Gt,
    Lt,
    Approx,
    Geq,
    Leq,

    Integral,
    Sum,
    Prod,
    Limit,
    Derivative,
    Partial,

    Sin,
    Cos,
    Tan,
    Cot,
    Sec,
    Cosec,

    Arcsin,
    Arccos,
    Arctan,

    Ln,
    Log,
    Exp,

    Abs,
    Floor,
    Ceil,
    Fact,
    Binom,

    Matrix,
    Vec,
    Det,
    Set,
    In,
    NotIn,

    Subset,
    Superset,
    Union,
    Intersect,

    And,
    Or,
    Not,
    Implies,
    Iff,
    ForAll,
    Exists,

    Paren,
    Bracket,
    Brace,
    // VBar,

    Text,
    Subscript,
    Superscript,
    Overline,

    Hat,
    Dot,
    Ddot,
    Arrow,
    Box,
}

pub enum Arguments {
    Fixed(usize),
    Range(usize, usize),
    Atleast(usize),
    Variadic, // 0..=usize::MAX
}

pub enum FuncKind {
    Native,
    UserDefined,
}

pub struct FuncMetadata {
    args: Arguments,
    kind: FuncKind,
}

impl ShowFunc {
    pub fn metadata(&self) -> FuncMetadata {
        todo!()
    }
}

impl From<String> for ShowFunc {
    fn from(s: String) -> Self {
        match s.as_str() {
            "define" => Self::Define,
            "defunc" => Self::Defunc,
            "quote" => Self::Quote,
            "string" => Self::String,
            "jux" | "juxtapose" => Self::Jux,
            "+" | "add" => Self::Add,
            "-" | "sub" => Self::Sub,
            "*" | "mul" => Self::Mul,
            "/" | "div" => Self::Div,
            "^" | "pow" => Self::Pow,
            "frac" => Self::Frac,
            "sqrt" => Self::Sqrt,
            "root" => Self::Root,
            "eq" | "=" => Self::Eq,
            "approx" | "≈" => Self::Approx,
            "neq" | "!=" => Self::Neq,
            "gt" | ">" => Self::Gt,
            "lt" | "<" => Self::Lt,
            "geq" | ">=" => Self::Geq,
            "leq" | "<=" => Self::Leq,
            "integral" => Self::Integral,
            "sum" => Self::Sum,
            "prod" | "product" => Self::Prod,
            "lim" | "limit" => Self::Limit,
            "derivative" | "deriv" => Self::Derivative,
            "partial" => Self::Partial,
            "sin" => Self::Sin,
            "cos" => Self::Cos,
            "tan" => Self::Tan,
            "cot" => Self::Cot,
            "sec" => Self::Sec,
            "cosec" => Self::Cosec,
            "sinh" | "arcsin" => Self::Arcsin,
            "cosh" | "arccos" => Self::Arccos,
            "tanh" | "arctan" => Self::Arctan,
            "ln" => Self::Ln,
            "log" => Self::Log,
            "exp" => Self::Exp,
            "abs" => Self::Abs,
            "floor" => Self::Floor,
            "ceil" => Self::Ceil,
            "fact" | "factorial" => Self::Fact,
            "binom" | "nCr" => Self::Binom,
            "mat" | "matrix" => Self::Matrix,
            "vec" | "vector" => Self::Vec,
            "det" | "determinant" => Self::Det,
            "set" => Self::Set,
            "in" => Self::In,
            "notin" => Self::NotIn,
            "subset" => Self::Subset,
            "superset" => Self::Superset,
            "union" => Self::Union,
            "intersect" => Self::Intersect,
            "and" => Self::And,
            "or" => Self::Or,
            "not" => Self::Not,
            "implies" => Self::Implies,
            "iff" => Self::Iff,
            "forall" => Self::ForAll,
            "exists" => Self::Exists,
            "paren" => Self::Paren,
            "bracket" => Self::Bracket,
            "brace" => Self::Brace,
            "text" => Self::Text,
            "subscript" => Self::Subscript,
            "superscript" => Self::Superscript,
            "bar" | "overline" => Self::Overline,
            "hat" => Self::Hat,
            "dot" => Self::Dot,
            "ddot" => Self::Ddot,
            "arrow" => Self::Arrow,
            "box" => Self::Box,
            _ => Self::MaybeFunc(s),
        }
    }
}

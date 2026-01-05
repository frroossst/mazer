use mazer_atog::Atog;
use mazer_types::LispAST;

pub trait ToMathML {
    fn to_mathml(&self) -> String;
}

impl ToMathML for LispAST {
    fn to_mathml(&self) -> String {
        match self {
            LispAST::Number(n) => format!("<mn>{}</mn>", n),
            
            LispAST::Symbol(s) => {
                // Check if it's a Greek letter or special symbol
                if let Some(entity) = Atog::get(s) {
                    format!("<mi>{}</mi>", entity)
                } else {
                    format!("<mi>{}</mi>", s)
                }
            }
            
            LispAST::String(s) => format!("<mtext>{}</mtext>", s),
            
            LispAST::Bool(b) => format!("<mtext>{}</mtext>", b),
            
            LispAST::List(exprs) if exprs.is_empty() => {
                "<mrow><mo>(</mo><mo>)</mo></mrow>".to_string()
            }
            
            LispAST::List(exprs) => {
                // Try to match mathematical operations
                if let Some(LispAST::Symbol(op)) = exprs.first() {
                    match op.as_str() {
                        "integral" => todo!(),
                        // Function application like f(x)
                        _ => format!("{:?}", self),
                    }
                } else {
                    // Generic list rendering
                    todo!()
                }
            }
            
            LispAST::Application { name, args } => {
                    todo!()
            }
            
            LispAST::Error(e) => {
                format!("<merror><mtext>{}</mtext></merror>", e)
            }
            
            LispAST::UserFunc { .. } | LispAST::NativeFunc(_) => {
                "<mtext>⟨function⟩</mtext>".to_string()
            }
        }
    }
       
}


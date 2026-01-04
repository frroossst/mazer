use mazer_types::LispAST;

pub trait ToMathML {
    fn to_mathml(&self) -> String;
}

impl ToMathML for LispAST {
    fn to_mathml(&self) -> String {
        String::from("Error")
    }
}


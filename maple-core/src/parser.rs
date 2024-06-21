use crate::tokenizer::Token;

#[derive(Debug)]
pub struct Parser {

}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        dbg!(&tokens);

        // collect all the Token::Let(Expr) into a vector
        let let_expr = tokens.iter().filter(|t| match t {
            Token::LetExpr(_, _) => true,
            _ => false,
        }).collect::<Vec<&Token>>();


        Parser {

        }
    }
}
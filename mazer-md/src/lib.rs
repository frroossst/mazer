use bumpalo::Bump;


pub enum Tokens {

}


pub struct Parser {
    pos: usize,
    bump: Bump,
}

impl Parser {
    pub fn new() -> Self {
        Parser {
            pos: 0,
            tokens: Vec<Tokens>::new(),
            bump: Bump::new(),
        }
    }
}


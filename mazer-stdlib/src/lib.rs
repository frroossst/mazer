use fastnum::D512;
use mazer_types::LispAST;


pub struct Native;

impl Native {

    pub fn add(arg: Vec<LispAST>) -> LispAST {
        todo!()
    }

    pub fn sub(arg: Vec<D512>) -> D512 {
        arg.iter().fold(D512::from(0), |acc, x| acc - *x)
    }

    pub fn mul(arg: Vec<D512>) -> D512 {
        arg.iter().fold(D512::from(1), |acc, x| acc * *x)
    }

    pub fn div(arg: Vec<D512>) -> D512 {
        arg.iter().fold(D512::from(1), |acc, x| acc / *x)
    }
}

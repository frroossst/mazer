use crate::parser::{LispErr, LispExpr};

#[allow(dead_code)]
pub struct Matrix {
    rows: usize,
    cols: usize,
    rvec: Vec<f64>,
    cvec: Vec<f64>,
}

impl Matrix {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            rvec: Vec::new(),
            cvec: Vec::new(),
        }
    }

    pub fn list_to_vector(list: &[LispExpr]) -> Result<Vec<f64>, LispErr> {
        let mut vec = Vec::new();
        for elem in list {
            if let LispExpr::Number(n) = elem {
                vec.push(*n);
            } else {
                return Err(LispErr::new("Vector elements must be numbers"));
            }
        }
        Ok(vec)
    }
}

use faer::prelude::*;

use crate::parser::{LispErr, LispExpr};

#[allow(dead_code)]
pub struct Matrix(Mat<f64>);

impl Matrix {
    pub fn new(rows: usize, cols: usize) -> Self {
        let mat = Mat::from_fn(rows, cols, |_, _| 0.0);
        Self(mat)
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

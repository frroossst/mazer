use faer::prelude::*;

pub struct Matrix(Mat<f64>);

impl Matrix {
    pub fn new(rows: usize, cols: usize) -> Self {
        let mat = Mat::from_fn(rows, cols, |_, _| 0.0);
        Self(mat)
    }
}

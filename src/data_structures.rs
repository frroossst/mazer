use std::fmt::Display;

use crate::vec;

pub struct Vector {
    dimensions: usize,
    elements: Vec<f64>,
}

impl Vector {
    pub fn new(dimensions: usize) -> Vector {
        Vector { 
            elements: Vec::with_capacity(dimensions),
            dimensions,
        }
    }

    
}

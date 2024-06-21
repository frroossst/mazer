// create a tagged union to represent the different types of data
#[derive(Debug, Clone)]
pub struct TaggedType<T> {
    tag: Tag,
    data: Data<T>,
}

#[derive(Debug, Clone)]
pub enum Tag {
    Directive,
    Function,
    Number,
    String,
    Variable,
}

#[derive(Debug, Clone)]
pub struct Data<T> {
    data: T,
}
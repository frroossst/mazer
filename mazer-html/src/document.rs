use mazer_parser::AST;

pub struct Document {
    head: String,
    body: Vec<String>,
    tail: String,
}

impl Document {

    pub fn new() -> Self {

        let head = "<!DOCTYPE html><html lang=\"en\"><meta charset=\"UTF-8\"><link rel=\"icon\" href=\"data:image/svg+xml,<svg xmlns=%22http://www.w3.org/2000/svg%22 viewBox=%220 0 32 32%22><text y=%2232%22 font-size=%2232%22>🍁</text></svg>\"><script src=\"https://cdn.jsdelivr.net/npm/@arborium/arborium/dist/arborium.iife.js\" data-theme=\"github-light\" data-selector=\"pre code\"></script>";

        Document {
            head: String::from(head),
            body: Vec::new(),
            tail: String::new(),
        }
    }

    fn append(&mut self, content: &str) {
        self.body.push(String::from(content));
    }

    pub fn append_node(&mut self, node: AST) {


    }

}

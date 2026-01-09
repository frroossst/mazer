use std::collections::BTreeMap;
use mazer_stdlib::{Native, Prelude};
use mazer_types::{LispAST, Environment};

use crate::{interpreter::Interpreter, parser::Parser};

type EnvMap = BTreeMap<String, LispAST>;

// Extension trait for Environment initialization
// These remain in mazer-lisp since they depend on Parser and Interpreter
pub trait EnvironmentExt {
    fn with_prelude(&mut self) -> Self;
    fn with_native(&mut self) -> Self;
}

impl EnvironmentExt for Environment {
    fn with_prelude(&mut self) -> Self {
        let prelude = Prelude::new();

        for (_k, v) in prelude {
            let mut parser = Parser::new(&v);
            let ast = parser.parse().expect("Failed to parse prelude function");
            let mut interp = Interpreter::new(BTreeMap::new(), Self { bindings: self.bindings.clone() });
            interp.eval(ast).expect("Failed to evaluate prelude function");
            self.bindings = interp.env().bindings.clone();
        }

        Self { bindings: self.bindings.clone() }
    }
    
    fn with_native(&mut self) -> Self {
        let mut env = EnvMap::new();
        
        // TODO: add more stdlib functions here
        env.insert("+".into(), mazer_types::LispAST::NativeFunc(Native::add));
        env.insert("add".into(), mazer_types::LispAST::NativeFunc(Native::add));
        env.insert("-".into(), mazer_types::LispAST::NativeFunc(Native::sub));
        env.insert("sub".into(), mazer_types::LispAST::NativeFunc(Native::sub));
        env.insert("*".into(), mazer_types::LispAST::NativeFunc(Native::mul));
        env.insert("mul".into(), mazer_types::LispAST::NativeFunc(Native::mul));
        env.insert("/".into(), mazer_types::LispAST::NativeFunc(Native::div));
        env.insert("div".into(), mazer_types::LispAST::NativeFunc(Native::div));

        env.insert("reflect".into(), mazer_types::LispAST::NativeFunc(Native::reflect));
        env.insert("print".into(), mazer_types::LispAST::NativeFunc(Native::print));
        env.insert("debug".into(), mazer_types::LispAST::NativeFunc(Native::debug));

        self.extend(&env);

        Self { bindings: env }
        
    }
}

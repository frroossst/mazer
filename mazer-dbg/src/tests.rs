#[cfg(test)]    
mod tests {

    mod mazer_dbg {
        pub use crate::init;
        pub use crate::inspect;
    }

    #[test]
    fn all_tests() {
        mazer_dbg::init();

        let x = 123;
        let y = "hello";
        let z = vec![1, 2, 3];

        mazer_dbg::inspect!(x, y, z);

        let a = vec![1, 2, 3, 4, 5];
        let b = std::collections::HashMap::from([
            ("key1", "value1"),
            ("key2", "value2"),
            ("key3", "value3"),
            ("key4", "value4"),
        ]);
        let c = std::collections::LinkedList::from([1, 2, 3, 4, 5]);

        mazer_dbg::inspect!(a, b, c);

        #[derive(Debug, Clone, Default)]
        struct Foo {
            x: i32,
            y: i32,
        }

        let p1 = Foo { x: 10, y: 20 };
        let p2 = Foo { x: 30, y: 40 };

        #[derive(Debug, Clone, Default)]
        struct Bar {
            name: String,
            value: f64,
        }

        let p3 = Bar {
            name: String::from("example"),
            value: 3.14,
        };

        #[derive(Debug, Clone, Default)]
        struct Baz {
            items: Vec<String>,
            count: usize,
        }

        let p4 = Baz {
            items: vec![String::from("apple"), String::from("banana")],
            count: 2,
        };

        #[derive(Debug, Clone)]
        struct Qux {
            foo: Foo,
            bar: Bar,
            baz: Baz,
            qux: Option<Box<Qux>>,
        }

        impl Default for Qux {
            fn default() -> Self {
                Qux {
                    foo: Foo::default(),
                    bar: Bar::default(),
                    baz: Baz::default(),
                    qux: None,
                }
            }
        }

        let p5 = Qux {
            foo: Foo { x: 1, y: 2 },
            bar: Bar {
                name: String::from("nested"),
                value: 42.0,
            },
            baz: Baz {
                items: vec![String::from("nested_item")],
                count: 1,
            },
            qux: Some(Box::new(Qux::default())),
        };

        let p6 = Qux {
            foo: Foo { x: 3, y: 4 },
            bar: Bar {
                name: String::from("another"),
                value: 2.71,
            },
            baz: Baz {
                items: vec![String::from("another_item")],
                count: 1,
            },
            qux: Some(Box::new(p5.clone())),
        };

        mazer_dbg::inspect!(p1, p2, p3, p4, p5, p6);
    }
}
# Foo

## Bar

### Baz

#### Qux

- Item 1
- Item 2
- Item 3

```python
if __name__ == "__main__":
    print("Hello, World!")
```

```rust
fn main() {
    println!("Hello, World!");
}
```

```scheme
(define (factorial n)
  (if (= n 0)
      1
      (* n (factorial (- n 1)))))
```

> This is a blockquote.

Check this out [google!](https://www.google.com)

I dont want to tell you my ||secret||.

I like to write `code` snippets.

---

-[ ] Task 1
-[x] Task 2
-[ ] Task 3
-[x] Task 4

---

> This is a blockquote ALSO

Check this out [google!](https://www.google.com)

I dont want to tell you my ||secret||. Aint no way

# New header

I wanna write something (and provide some context).

I like math such that (show (+ 1 1)) equals to (eval (+ 1 1)).

# Math Demo with Show

Here are some mathematical expressions using the show command:

**Power expressions:**
The expression (show (pow x 2)) represents x squared.
And (show (pow x (pow y z))) shows nested exponents.

**Fractions:**
One half is (show (frac 1 2)).
A more complex fraction: (show (frac (+ a b) (- c d))).

**Square roots:**
The square root of x is (show (sqrt x)).
The cube root of x is (show (root 3 x)).
The famous formula: (show (= x (frac (+ (- b) (sqrt (- (pow b 2) (* 4 a c)))) (* 2 a)))).

**Calculus:**
An indefinite integral: (show (integral f)).
A definite integral: (show (integral 0 infinity (exp (- (pow x 2))) x)).
A sum: (show (sum (= i 1) n (pow i 2))).
A derivative: (show (deriv f x)).

**Greek letters:**
(show (= (+ alpha beta) gamma)).

**Trigonometry:**
(show (= (+ (pow (sin theta) 2) (pow (cos theta) 2)) 1)).

**Matrices:**
(show (matrix (1 2) (3 4))).

**Logic:**
(show (implies p q)).
(show (forall x (exists y (= x y)))).

```scheme
(eval
    (print (reflect +))
)

(eval
    (begin
        (define s (string hello))
        (print s)
    )
)

(eval
    (print (string 42))
)

(eval
    (print (string true))
)

(eval
    (print (string (quote (H e l l o))))
)

(eval
    (print (reflect (string world)))
)

(eval
    (print (string 👋🌍))
)
```



- Item A


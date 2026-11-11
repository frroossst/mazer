# The math side of Mazer

A note is markdown with Lisp forms in it. A parenthesis is code only when it opens with `show`, `eval`, `let`, `alias`, `macro` or `ref`. Everything else in parentheses is prose. This page shows each feature as source in a code block, followed by the same form live. It is compiled by the test suite, so every live form here must produce no diagnostics. The markdown side is in `markdown.md`.

## Show: inline and display

`show` renders its argument as math. Alone in its paragraph it is a display equation, centred on its own line. Inside a sentence it is inline math.

```scheme
(show (= (^ x 2) (* x x)))
```

(show (= (^ x 2) (* x x)))

Inside a sentence, like (show (= (+ a b) c)) here, the form is inline. A form on a line of its own directly under text is inline too; it needs blank lines around it to become a display equation.

Several expressions after `show` are written next to each other, as if wrapped in `jux`. The next two have the same result.

```scheme
(show a b c)
```

(show a b c)

```scheme
(show (jux a b c))
```

(show (jux a b c))

## Numbers, variables and display names

A number is shown as written. A name that is not a keyword is a variable and is shown as itself.

```scheme
(show (= x 3.14))
```

(show (= x 3.14))

Some names are shown as a symbol: Greek letters, `infinity`, `hbar` and a few arrows. This is only how they look; the name stays an ordinary variable.

```scheme
(show (= (+ alpha beta) gamma))
```

(show (= (+ alpha beta) gamma))

```scheme
(show (set alpha beta gamma delta epsilon theta lambda mu pi sigma phi omega))
```

(show (set alpha beta gamma delta epsilon theta lambda mu pi sigma phi omega))

```scheme
(show (set Gamma Delta Theta Lambda Sigma Phi Omega))
```

(show (set Gamma Delta Theta Lambda Sigma Phi Omega))

```scheme
(show (= E (* hbar omega)))
```

(show (= E (* hbar omega)))

```scheme
(show (< (- infinity) x infinity))
```

(show (< (- infinity) x infinity))

```scheme
(show (set rightarrow leftarrow Rightarrow Leftrightarrow mapsto))
```

(show (set rightarrow leftarrow Rightarrow Leftrightarrow mapsto))

Names such as `in`, `set`, `text` and `box` are keywords. To show one of those words as a plain variable, quote it: (show 'in) and (show 'text).

## Function application

A list whose head is not a keyword is a function application. The head is written as a name and the arguments in parentheses.

```scheme
(show (f x))
```

(show (f x))

```scheme
(show (= (f x y) (g (+ x 1) (h y))))
```

(show (= (f x y) (g (+ x 1) (h y))))

With no arguments the list renders just the name.

```scheme
(show (dirac))
```

(show (dirac))

A head that is one typo away from a keyword gets a hint (in the editor and on the command line), not a warning, since the name may be deliberate.

## Arithmetic and fractions

`+`, `-`, `*` and `/` take any number of operands, and `^` takes a base and an exponent. `*` is shown as a cross, a lone `-` negates, and `jux` writes its arguments side by side.

```scheme
(show (+ a b c))
```

(show (+ a b c))

```scheme
(show (- a b))
```

(show (- a b))

```scheme
(show (- a))
```

(show (- a))

```scheme
(show (* a b c))
```

(show (* a b c))

```scheme
(show (jux 2 x y))
```

(show (jux 2 x y))

```scheme
(show (frac (+ a b) (- c d)))
```

(show (frac (+ a b) (- c d)))

```scheme
(show (/ a b))
```

(show (/ a b))

Second names work as well: `add`, `sub`, `mul`, `div`, `pow`.

## Powers and roots

```scheme
(show (^ x 2))
```

(show (^ x 2))

```scheme
(show (^ e (- (^ x 2))))
```

(show (^ e (- (^ x 2))))

```scheme
(show (sqrt (+ (^ a 2) (^ b 2))))
```

(show (sqrt (+ (^ a 2) (^ b 2))))

```scheme
(show (root 3 x))
```

(show (root 3 x))

## Relations

```scheme
(show (= a b c))
```

(show (= a b c))

```scheme
(show (!= x 0))
```

(show (!= x 0))

```scheme
(show (< a b))
```

(show (< a b))

```scheme
(show (<= 0 x 1))
```

(show (<= 0 x 1))

```scheme
(show (>= x y))
```

(show (>= x y))

```scheme
(show (approx pi (/ 22 7)))
```

(show (approx pi (/ 22 7)))

## Sets

```scheme
(show (set 1 2 3))
```

(show (set 1 2 3))

```scheme
(show (in x (intersect A B)))
```

(show (in x (intersect A B)))

```scheme
(show (notin 0 (set 1 2)))
```

(show (notin 0 (set 1 2)))

```scheme
(show (subset A (union B C)))
```

(show (subset A (union B C)))

```scheme
(show (superset A B))
```

(show (superset A B))

## Logic and quantifiers

```scheme
(show (implies (and p q) (or r (not s))))
```

(show (implies (and p q) (or r (not s))))

```scheme
(show (iff p q))
```

(show (iff p q))

```scheme
(show (forall x (exists y (< x y))))
```

(show (forall x (exists y (< x y))))

A quantifier's body runs to the right. A quantifier used as an operand is parenthesised.

```scheme
(show (and (forall x (P x)) Q))
```

(show (and (forall x (P x)) Q))

## Sums, products, integrals, limits and derivatives

`sum`, `prod` and `integral` take the body and the variable first, both required. The bounds are optional and come last: `(integral body var)` or `(integral body var lower upper)`.

```scheme
(show (integral (sin x) x))
```

(show (integral (sin x) x))

```scheme
(show (integral (exp (- (^ x 2))) x (- infinity) infinity))
```

(show (integral (exp (- (^ x 2))) x (- infinity) infinity))

```scheme
(show (sum (frac 1 (^ k 2)) k 1 infinity))
```

(show (sum (frac 1 (^ k 2)) k 1 infinity))

```scheme
(show (prod (frac k (+ k 1)) k 1 n))
```

(show (prod (frac k (+ k 1)) k 1 n))

The lower bound may also be a condition or a set that the variable ranges over.

```scheme
(show (sum (f i) i (in i S)))
```

(show (sum (f i) i (in i S)))

`limit` (or `lim`) takes the variable, the point and the body. A fourth argument is a one-sided limit.

```scheme
(show (lim x 0 (frac (sin x) x)))
```

(show (lim x 0 (frac (sin x) x)))

```scheme
(show (lim x 0 (f x) +))
```

(show (lim x 0 (f x) +))

`derivative` takes the function and the variable, and optionally the order. `partial` takes only the function and the variable and writes the partial sign (it has no order). `dd` writes a Leibniz ratio of differentials, dy/dx, and also takes an optional order.

```scheme
(show (derivative (sin x) x))
```

(show (derivative (sin x) x))

```scheme
(show (derivative f x 2))
```

(show (derivative f x 2))

```scheme
(show (partial f x))
```

(show (partial f x))

```scheme
(show (dd y x))
```

(show (dd y x))

```scheme
(show (evalat (frac 1 x) 2))
```

(show (evalat (frac 1 x) 2))

## Trigonometry, logarithms and other functions

```scheme
(show (= (+ (^ (sin theta) 2) (^ (cos theta) 2)) 1))
```

(show (= (+ (^ (sin theta) 2) (^ (cos theta) 2)) 1))

```scheme
(show (tan (+ x 1)))
```

(show (tan (+ x 1)))

```scheme
(show (ln (exp x)))
```

(show (ln (exp x)))

```scheme
(show (log 2 x))
```

(show (log 2 x))

```scheme
(show (abs (- a b)))
```

(show (abs (- a b)))

```scheme
(show (floor (frac n 2)))
```

(show (floor (frac n 2)))

```scheme
(show (fact (+ n 1)))
```

(show (fact (+ n 1)))

```scheme
(show (binom n k))
```

(show (binom n k))

## Grouping and automatic parentheses

Parentheses are added by precedence, so you rarely write them. A sum inside a product gets them; a product inside a sum does not.

```scheme
(show (* (+ a b) c))
```

(show (* (+ a b) c))

```scheme
(show (+ (* a b) c))
```

(show (+ (* a b) c))

```scheme
(show (^ (+ a b) n))
```

(show (^ (+ a b) n))

```scheme
(show (- a (+ b c)))
```

(show (- a (+ b c)))

```scheme
(show (+ a (- b)))
```

(show (+ a (- b)))

`paren`, `bracket` and `brace` write the fences explicitly.

```scheme
(show (jux f (paren x) (bracket y) (brace z)))
```

(show (jux f (paren x) (bracket y) (brace z)))

## Decorations

```scheme
(show (subscript x i))
```

(show (subscript x i))

```scheme
(show (superscript x (+ n 1)))
```

(show (superscript x (+ n 1)))

```scheme
(show (bar x))
```

(show (bar x))

```scheme
(show (hat v))
```

(show (hat v))

```scheme
(show (dot q))
```

(show (dot q))

```scheme
(show (ddot q))
```

(show (ddot q))

```scheme
(show (arrow v))
```

(show (arrow v))

```scheme
(show (prime f))
```

(show (prime f))

```scheme
(show (prime f 2))
```

(show (prime f 2))

```scheme
(show (box (= E (* m (^ c 2)))))
```

(show (box (= E (* m (^ c 2)))))

```scheme
(show (= x (text for all n)))
```

(show (= x (text for all n)))

```scheme
(show (funcapp f x))
```

(show (funcapp f x))

`funcapp` is the long form of function application, useful when the head is itself an expression.

```scheme
(show (funcapp (+ f g) x))
```

(show (funcapp (+ f g) x))

## Matrices and determinants

```scheme
(show (matrix (1 0) (0 1)))
```

(show (matrix (1 0) (0 1)))

```scheme
(show (det (a b) (c d)))
```

(show (det (a b) (c d)))

```scheme
(show (vec x y z))
```

(show (vec x y z))

## Cases

Each clause is a list holding a value and, optionally, a condition.

```scheme
(show (= (abs x) (cases (x (>= x 0)) ((- x) (text otherwise)))))
```

(show (= (abs x) (cases (x (>= x 0)) ((- x) (text otherwise)))))

## Tables

`table` takes a `header` row and `row` rows, and optionally `style` and `align` directives. The styles are `grid` (the default: an outer frame and a line between every row and column), `clean` (an outer frame and a rule under the header) and `plain` (no lines at all). A note on browsers: Chromium-based browsers currently ignore the table lines and the bold header that MathML asks for, so in them a table shows without rules (a browser limitation, not a mazer one).

```scheme
(show (table (header Name Age) (row Alice 30) (row Bob 25)))
```

(show (table (header Name Age) (row Alice 30) (row Bob 25)))

```scheme
(show (table (style clean) (align left right) (header n (frac 1 n)) (row 1 1) (row 2 (frac 1 2))))
```

(show (table (style clean) (align left right) (header n (frac 1 n)) (row 1 1) (row 2 (frac 1 2))))

## Inference rules

`ty-judge` takes a rule name, a list of premises, and a conclusion. A single premise must still be wrapped in a list.

```scheme
(show (ty-judge T-Var ((in x S)) (= x x)))
```

(show (ty-judge T-Var ((in x S)) (= x x)))

```scheme
(show (ty-judge T-Add ((= a b) (= c d)) (= (+ a c) (+ b d))))
```

(show (ty-judge T-Add ((= a b) (= c d)) (= (+ a c) (+ b d))))

## Font variants

```scheme
(show (set (bb R) (cal L) (frak g) (bold v) (rm d)))
```

(show (set (bb R) (cal L) (frak g) (bold v) (rm d)))

## Quote

`quote`, written `'x`, does two things. It shows a keyword's name instead of its sign, which matters for words that are also keywords, and it stops `let` substitution, so a variable is shown by its name instead of its value. It does not stop notation: an alias or a macro inside a quote is still expanded.

```scheme
(show (= 'sum (+ 1 2)))
```

(show (= 'sum (+ 1 2)))

```scheme
(show 'integral)
```

(show 'integral)

Here the alias `Field` is defined, and `zq` is bound with `let`. The quoted `'Field` is still replaced by the alias body, while the quoted `'zq` keeps its name and the unquoted one shows its value.

```scheme
(alias Field (bb F))
```

(alias Field (bb F))

```scheme
(let zq 5)
```

(let zq 5)

```scheme
(show (= 'Field Field))
```

(show (= 'Field Field))

```scheme
(show (= 'zq zq))
```

(show (= 'zq zq))

## Strings

A double-quoted string is shown as upright text, exactly as written: spaces and punctuation are kept, and a string is never read as a keyword or a variable. Use one as an argument anywhere, or inside `text`, which joins its words with single spaces.

```scheme
(show (= x "hello world"))
```

(show (= x "hello world"))

```scheme
(show (text "if x > 0, then"))
```

(show (text "if x > 0, then"))

```scheme
(show (cases (1 "x is even") (0 "x is odd")))
```

(show (cases (1 "x is even") (0 "x is odd")))

```scheme
(show (set "sum" "in" sum))
```

(show (set "sum" "in" sum))

The strings `"sum"` and `"in"` stay words, while the bare `sum` is the sign. A string is also how to keep a semicolon or parenthesis: inside quotes they are not code.

```scheme
(show (= "a;b" "(x)"))
```

(show (= "a;b" "(x)"))

## A bare keyword shows its sign

A keyword written without parentheses renders its symbol. Operators, relations, big operators and named functions have one; layouts such as `frac` or `matrix` do not, and using one bare is a diagnostic.

```scheme
(show (set integral sum prod + = in forall sin))
```

(show (set integral sum prod + = in forall sin))

## Comments

A semicolon inside a form starts a comment that runs to the end of the line, and it also ends the atom before it. A comment exists only inside a code form, so it must come before the form's closing parenthesis, and the parenthesis then goes on a later line. Outside a form, in prose, a semicolon is just a character. In the form below both comments are inside it, and neither appears in the output.

```scheme
(show
    (+ a    ; the first term
       b    ; the second term
    ))
```

(show
    (+ a    ; the first term
       b    ; the second term
    ))

## Notation: alias and macro

`alias` gives a name to an expression. `macro` takes parameters, and may have a doc string right after the signature, which the editor shows on hover. A parameter spelled `name...` as the last one collects the remaining arguments; in the body it is written without the dots and must stand inside a list. Definitions produce no output.

Definitions apply to the whole note, wherever they are written, and a macro may use other macros. Substitution is structural: the arguments go into the body exactly as written, and nothing is evaluated.

```scheme
(alias Nat (bb N))
```

(alias Nat (bb N))

```scheme
(show (in n Nat))
```

(show (in n Nat))

```scheme
(macro (laplace f s)
    "The Laplace transform of f, at s."
    (jux Laplace (brace f) (paren s)))
```

(macro (laplace f s)
    "The Laplace transform of f, at s."
    (jux Laplace (brace f) (paren s)))

```scheme
(show (= (laplace f s) (integral (jux (f t) (exp (- (* s t)))) t 0 infinity)))
```

(show (= (laplace f s) (integral (jux (f t) (exp (- (* s t)))) t 0 infinity)))

```scheme
(macro (members first rest...)
    (set first rest))
```

(macro (members first rest...)
    (set first rest))

```scheme
(show (members a b c d))
```

(show (members a b c d))

Reusing the name of a keyword or a display name for a macro is an error.

## Equation numbers: tag and ref

`(tag name)` as the first item of a display `show` numbers the equation. `(ref name)` in prose renders its number as a link, and may come before the equation. The next paragraph refers forward to equation (ref euler), which is written below it.

```scheme
As shown in (ref euler), ...
```

```scheme
(show (tag euler) (= (^ e (* i pi)) (- 1)))
```

(show (tag euler) (= (^ e (* i pi)) (- 1)))

Equations are numbered in document order, and only the tagged ones. A second tagged equation follows. Here (ref euler) is Euler's identity and (ref sqrt2) is the equation below.

```scheme
(show (tag sqrt2) (= (^ (sqrt 2) 2) 2))
```

(show (tag sqrt2) (= (^ (sqrt 2) 2) 2))

## Variables: let

`let` binds a variable. It is different from `alias`: the value is captured when the `let` is read, a variable can be rebound any number of times, and `show` substitutes the current value wherever the name appears afterwards. Quote a name to keep it as it is. The binding stays in force for the rest of the note, so this section comes last.

```scheme
(let x 2)
(show x)          ; 2
(show (+ x 1))    ; 2 + 1
(eval (+ x 1))    ; 3
(let y (+ x 1))   ; y holds (+ 2 1)
(let x 3)         ; reassigned
(show (= 'x x))   ; x = 3
```

Live, without the `eval` line:

(let x 2)

(show x)

(show (+ x 1))

(let y (+ x 1))

(let x 3)

(show (= 'x x))

After the rebinding `y` still holds the old value, because it was captured at the `let`: (show y).

## Evaluation

`eval` asks an evaluator to compute a form and shows the result. No evaluator ships yet, so a live `eval` would render an error box. It is written like this, and must stand alone, never inside a `show`.

```scheme
(eval (+ 1 2))
```

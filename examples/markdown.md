# A tour of the markdown side

This file shows every markdown feature a note can use. Each feature has a one-line explanation, and most are followed by the source that produced them. The math side is in `docs.md`.

## Headings

Six levels, `#` to `######`. A heading may contain inline math: a form written with `show` inside the sentence.

# Level one
## Level two
### Level three
#### Level four
##### Level five
###### Level six

### The value of (show (= pi 3.14159)) in a heading

## Paragraphs and line breaks

A blank line starts a new paragraph.

A single newline inside a paragraph is a soft break,
so this sentence continues on a new line.

Two spaces at the end of a line make a hard break.  
This line follows a hard break.

A single backslash at the end of a line does the same.\
This line follows a backslash break.

## Text styles

Wrap text in `**two stars**` for **bold**, in `*one star*` for *italic*, in `_underscores_` for _underline_, and in `~tildes~` for ~strikethrough~ (`~~two~~` also works). Each pair needs its closing marker in the same paragraph, or the markers stay as literal text.

Styles nest: **bold with *italic inside* and _underline too_**, and ~strikethrough around **bold**~.

A star directly after a letter or digit is a literal star, so `lambda*I` stays as it is: lambda*I.

A backslash makes any punctuation literal: \*not italic\*.

## Spoilers

Wrap text in `||two bars||` to hide it behind a dark box. The text shows while the mouse pointer is over the box and hides again when the pointer leaves; the reveal is hover only and has no click handler. I dont want to tell you my ||secret||.

## Links and images

A link is `[text](url)`, and the text may be styled: [**bold** google](https://www.google.com).

An image on a line of its own becomes a block:

![a placeholder picture](https://example.com/picture.png)

An image inside a sentence stays inline: ![small](https://example.com/small.png) like this.

## Code

Inline code uses backticks and is never interpreted: `(show (+ 1 1))` stays as text, and so does `**not bold**`.

A fenced block takes the language after the opening fence. Nothing inside it is interpreted either.

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

## Block quotes

A block quote starts each line with `>`. It holds other blocks.

> This is a quote with **bold** text.
>
> - a list inside the quote
> - another item
>
> A second paragraph, with math: (show (= (^ e (* i pi)) -1)).

## Lists

A bullet list uses `-`, `*` or `+`.

- Item 1
- Item 2
- Item 3

A numbered list uses `1.` or `1)`. The first number is the starting number.

1. First
2. Second
3. Third

Start somewhere else:

5. Five
6. Six

Indent an item by the width of its marker to nest a list under it.

- Fruit
  - Apple
  - Pear
    1. Green
    2. Yellow
- Vegetables
  - Carrot

A checklist item is `-[ ]` or `-[x]`. The GitHub spelling `- [ ]` works too.

-[ ] Task 1
-[x] Task 2
-[ ] Task 3
-[x] Task 4

## Rules

Three dashes alone on a line draw a horizontal rule.

---

## Emoji

A shortcode between colons becomes an emoji. There are exactly thirteen, and unknown ones stay as text.

:sob: :maple: :heart: :eyes: :thinking: :+1: :-1: :brain: :fire: :sparkles: :rocket: :coffee: :sunglasses:

An unknown name like :banana: is left alone.

## Prose parentheses and code

Only a parenthesis that opens with one of six words is code: `show`, `eval`, `let`, `alias`, `macro` and `ref`. Every other parenthesis is prose, so (this is just a sentence) and (so is f(x) = 2x).

A `show` form inside a sentence renders as inline math: the sum (show (+ a b)) and the fraction (show (frac 1 2)).

A form alone in its paragraph is a display equation:

(show (= (^ x 2) (* x x)))

If a sentence happens to begin a parenthesis with a reserved word, put a backslash in front of the paren. Compare \(show that f is continuous) with the same words written in code.

A form may run over several lines, but it never crosses a blank line.

## Math samples

These are the same forms as in `docs.md`, in short.

The expression (show (^ x 2)) is x squared, and (show (^ x (^ y z))) is a nested power.

One half is (show (frac 1 2)). A larger fraction: (show (frac (+ a b) (- c d))).

The square root of x is (show (sqrt x)), and the cube root is (show (root 3 x)).

The quadratic formula:

(show (= x (frac (+ (- b) (sqrt (- (^ b 2) (* 4 a c)))) (* 2 a))))

An integral with respect to x: (show (integral (^ x 2) x)). A definite one: (show (integral (exp (- (^ x 2))) x 0 infinity)).

A sum: (show (sum (^ i 2) i 1 n)). A derivative: (show (derivative f x)).

A Greek identity: (show (= (+ alpha beta) gamma)).

(show (= (+ (^ (sin theta) 2) (^ (cos theta) 2)) 1))

A matrix:

(show (matrix (1 2) (3 4)))

Logic: (show (implies p q)) and (show (forall x (exists y (= x y)))).

A table with a bold header row, in the default `grid` style, which draws an outer frame and a line between every row and column (`clean` draws the frame and a rule under the header, `plain` no lines). A note on browsers: Chromium-based browsers currently ignore the table lines and the bold header that MathML asks for, so in them a table shows without rules (a browser limitation, not a mazer one).

(show (table (header Name Age City) (row Alice 30 NYC) (row Bob 25 LA)))

A `clean` table, with an outer frame and a rule under the header, right-aligned values and math in the cells:

(show (table (style clean) (align left right) (header n (frac 1 n)) (row 1 1) (row 2 (frac 1 2))))

Evaluation with `eval` needs an evaluator, which this build does not ship, so there is no live example here.

# In a document

- let reserves an expression block until a semi-colon is encountered.
maybe no semicolon only newlines

- fmt() macro expands the symbol to it's MathML expression

- eval() macro evaluates the expression passed to it

- `//` to mark comments, only single line comments are allowed

```
let itg = integral(0, "t", "f(x)")

fmt(itg) // replaces the function call with the MathML expression

eval(itg) // replaces the function call with the answer of the expression
```

# Simple declarations

```
let x = 15;

let y = 1 + 1;
```

# InBuilt Functions

```
print(x); // prints the value of x

eval(y); // evaluates the value of y and returns it
```

# Constants

```
pi = 3.14;

e = 2.71;

i = sqrt(-1);

inf = 1/0; // infinity

NaN = 0/0; // Not a Number
```

# Math Functions

```
abs(x); // returns the absolute value of x

sqrt(x); // returns the square root of x

pow(x, y); // returns x raised to the power of y

log(x); // returns the natural logarithm of x

ln(x); // returns the natural logarithm of x

exp(x); // returns e raised to the power of x

sin(x); // returns the sine of x

cos(x); // returns the cosine of x

tan(x); // returns the tangent of x

inv(x); // returns the inverse of x

```


# Data Structures

## Arrays 
``` 
let arr = [1, 2, 3, 4, 5];
``` 

## Matrix
```
let mat = [[1, 2, 3], [4, 5, 6], [7, 8, 9]];

let m = [
    [1, 2, 3],
    [4, 5, 6],
    ];
```




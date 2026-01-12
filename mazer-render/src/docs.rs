/// Formats an addition expression.
/// 
/// # Arguments
/// - N args: `(add expr1 expr2 ... exprN)` - adds all expressions
/// # Examples
/// ```scheme
/// (show (add 1 2 3))
/// (show (add 11 12))
/// ```
#[doc (alias = "+")]
pub mod add {}

/// Formats an integral expression.
///
/// # Arguments
/// - 1 arg: `(integral expr)` - unbounded integral without differential
/// - 2 args: `(integral expr var)` - indefinite integral: ∫ expr dvar
/// - 3 args: `(integral lower upper expr)` - definite integral without explicit differential
/// - 4 args: `(integral lower upper expr var)` - definite integral with differential
///
/// # Examples
/// ```scheme
/// (show (integral f))
/// (show (integral (pow x 2) x))
/// (show (integral 0 1 (pow x 2)))
/// (show (integral 0 infinity (exp (- (pow x 2))) x))
/// ```
pub mod integral {}

/// Formats a summation expression.
///
/// # Arguments
/// - 1 arg: `(sum expr)` - unbounded summation
/// - 3 args: `(sum lower upper expr)` - summation with bounds
///
/// # Examples
/// ```scheme
/// (show (sum (pow i 2)))
/// (show (sum (= i 1) n (pow i 2)))
/// ```
#[doc(alias = "summation")]
pub mod sum {}



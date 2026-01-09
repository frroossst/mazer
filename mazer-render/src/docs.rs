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



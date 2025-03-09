#[macro_export]
macro_rules! ivec {
    () => {
        "<mover>
            <mi>i</mi>
            <mo>&#x2192;</mo>
        </mover>"
    };
}

#[macro_export]
macro_rules! jvec {
    () => {
        "<mover>
            <mi>j</mi>
            <mo>&#x2192;</mo>
        </mover>"
    };
}

#[macro_export]
macro_rules! kvec {
    () => {
        "<mover>
            <mi>k</mi>
            <mo>&#x2192;</mo>
        </mover>"
    };
}

#[macro_export]
macro_rules! icap {
    () => {
        "
        <mover>
            <mi>i</mi>
            <mo>&#x0302;</mo>
        </mover>"
    };
}

#[macro_export]
macro_rules! jcap {
    () => {
        "
        <mover>
            <mi>j</mi>
            <mo>&#x0302;</mo>
        </mover>"
    };
}

#[macro_export]
macro_rules! kcap {
    () => {
        "
        <mover>
            <mi>k</mi>
            <mo>&#x0302;</mo>
        </mover>"
    };
}

/// vec!(x) takes in a string and
/// returns a vector representation of the string
#[macro_export]
macro_rules! vec {
    ($x:expr) => {
        format!("<mover><mi>{}</mi><mo>&#x2192;</mo></mover>", $x)
    };
}

#[macro_export]
macro_rules! cap {
    ($x: expr) => {
        format!("<mover><mi>{}</mi><mo>&#x0302;</mo></mover>", $x)
    };
}

#[macro_export]
macro_rules! matrix {
    ( $( [ $( $x:expr ),* ] ),* ) => {{
        let mut rows = String::new();
        $(
            let mut row = String::new();
            $(
                row.push_str(&format!("<mtd>{}</mtd>", $x));
            )*
            rows.push_str(&format!("<mtr>{}</mtr>", row));
        )*
        format!("<mfenced open='[' close=']'><mo>[</mo><mtable>{}</mtable><mo>]</mo>", rows)
    }};
}

/// definite integral
#[macro_export]
macro_rules! defintegral {
    ($a:expr, $b:expr, $f:expr, $dx:expr) => {
        format!("<msubsup><mo stretchy=\"true\" largeop=\"true\">&#x222B;</mo><mn>{}</mn><mn>{}</mn></msubsup><mrow><mi>{}</mi><mo>&#x2062;</mo><mi>{}</mi></mrow>", $a, $b, $f, $dx)
    };
}

/// indefinite integral
#[macro_export]
macro_rules! integral {
    ($f:expr, $dx:expr) => {
        format!("<msubsup><mo stretchy=\"true\" largeop=\"true\">&#x222B;</mo><mo></mo><mo></mo></msubsup><mrow><mi>{}</mi><mo>&#x2062;</mo><mi>{}</mi></mrow>", $f, $dx)
    };
}

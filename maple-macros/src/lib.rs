/// VECTORS
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

/// CAPPED VECTORS
#[macro_export]
macro_rules! icap {
    () => {
        "
        <mover>
            <mi>i</mi>
            <mo>&#x0302;</mo>
        </mover>"
    }

}

#[macro_export]
macro_rules! jcap {
    () => {
        "
        <mover>
            <mi>j</mi>
            <mo>&#x0302;</mo>
        </mover>"
    }

}

#[macro_export]
macro_rules! kcap {
    () => {
        "
        <mover>
            <mi>k</mi>
            <mo>&#x0302;</mo>
        </mover>"
    }

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

/// MATRIX
/*
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
} */


/// INTEGRALS
#[macro_export]
macro_rules! defintegral {
    ($a:expr, $b:expr, $f:expr, $dx:expr) => {
        format!("<msubsup><mo stretchy=\"true\" largeop=\"true\">&#x222B;</mo><mn>{}</mn><mn>{}</mn></msubsup><mrow>{}<mo>&#x2062;</mo><mi>{}</mi></mrow>", $a, $b, $f, $dx)
    };
}

#[macro_export]
macro_rules! integral {
    ($f:expr, $dx:expr) => {
        format!("<msubsup><mo stretchy=\"true\" largeop=\"true\">&#x222B;</mo><mo></mo><mo></mo></msubsup><mrow>{}<mo>&#x2062;</mo><mi>{}</mi></mrow>", $f, $dx)
    };
}


/// OPERATORS
#[macro_export]
macro_rules! exponent {
    ($base:expr, $expo:expr) => {
        // check if base is already tagged with <mi> if it is then don't tag it again
        // check if it starts with mi and ends with mi
        if $base.starts_with("<mi") && $base.ends_with("</mi>") {
            format!("<msup>{}<mn>{}</mn></msup>", $base, $expo)
        } else {
            format!("<msup><mi>{}</mi><mn>{}</mn></msup>", $base, $expo)
        }
    };
}

#[macro_export]
macro_rules! fraction {
    ($base:expr, $expo:expr) => {
        // check if base is already tagged with <mi> if it is then don't tag it again
        // check if it starts with mi and ends with mi
        if $base.starts_with("<mi") && $base.ends_with("</mi>") {
            format!("<mfrac>{}<mn>{}</mn></mfrac>", $base, $expo)
        } else {
            format!("<mfrac><mi>{}</mi><mn>{}</mn></mfrac>", $base, $expo)
        }
    };
}


/// SYMBOLS
#[macro_export]
macro_rules! realNum {
    () => {
        "<mi mathvariant=\"double-struck\">R</mi>"
    };
}

#[macro_export]
macro_rules! thereExists {
    () => {
        "<mo>&#x2203;</mo>"
    };
}

#[macro_export]
macro_rules! forAll {
    () => {
        "<mo>&#x2200;</mo>"
    };
}

#[macro_export]
macro_rules! infinity {
    () => {
        "<mo>&#x221E;</mo>"
    };
}

#[macro_export]
macro_rules! angle {
    () => {
        "<mo>&#x2220;</mo>"
    };
}

#[macro_export]
macro_rules! degrees {
    () => {
        "<mo>&#x00B0;</mo>"
    };
}

#[macro_export]
macro_rules! Alpha {
    () => {
        "<mi>&#x0391;</mi>"
    };
}

#[macro_export]
macro_rules! alpha {
    () => {
        "<mi>&#x03B1;</mi>"
    };
}

#[macro_export]
macro_rules! Beta {
    () => {
        "<mi>&#x0392;</mi>"
    };
}

#[macro_export]
macro_rules! beta {
    () => {
        "<mi>&#x03B2;</mi>"
    };
}

#[macro_export]
macro_rules! Gamma {
    () => {
        "<mi>&#x0393;</mi>"
    };
}

#[macro_export]
macro_rules! gamma {
    () => {
        "<mi>&#x03B3;</mi>"
    };
}

#[macro_export]
macro_rules! Delta {
    () => {
        "<mi>&#x0394;</mi>"
    };
}

#[macro_export]
macro_rules! delta {
    () => {
        "<mi>&#x03B4;</mi>"
    };
}

#[macro_export]
macro_rules! Epsilon {
    () => {
        "<mi>&#x0395;</mi>"
    };
}

#[macro_export]
macro_rules! epsilon {
    () => {
        "<mi>&#x03B5;</mi>"
    };
}

#[macro_export]
macro_rules! Zeta {
    () => {
        "<mi>&#x0396;</mi>"
    };
}

#[macro_export]
macro_rules! zeta {
    () => {
        "<mi>&#x03B6;</mi>"
    };
}

#[macro_export]
macro_rules! Eta {
    () => {
        "<mi>&#x0397;</mi>"
    };
}

#[macro_export]
macro_rules! eta {
    () => {
        "<mi>&#x03B7;</mi>"
    };
}

#[macro_export]
macro_rules! Theta {
    () => {
        "<mi>&#x0398;</mi>"
    };
}

#[macro_export]
macro_rules! theta {
    () => {
        "<mi>&#x03B8;</mi>"
    };
}

#[macro_export]
macro_rules! Iota {
    () => {
        "<mi>&#x0399;</mi>"
    };
}

#[macro_export]
macro_rules! iota {
    () => {
        "<mi>&#x03B9;</mi>"
    };
}

#[macro_export]
macro_rules! Kappa {
    () => {
        "<mi>&#x039A;</mi>"
    };
}

#[macro_export]
macro_rules! kappa {
    () => {
        "<mi>&#x03BA;</mi>"
    };
}

#[macro_export]
macro_rules! Lambda {
    () => {
        "<mi>&#x039B;</mi>"
    };
}

#[macro_export]
macro_rules! lambda {
    () => {
        "<mi>&#x03BB;</mi>"
    };
}

#[macro_export]
macro_rules! Mu {
    () => {
        "<mi>&#x039C;</mi>"
    };
}

#[macro_export]
macro_rules! mu {
    () => {
        "<mi>&#x03BC;</mi>"
    };
}

#[macro_export]
macro_rules! Nu {
    () => {
        "<mi>&#x039D;</mi>"
    };
}

#[macro_export]
macro_rules! nu {
    () => {
        "<mi>&#x03BD;</mi>"
    };
}

#[macro_export]
macro_rules! Xi {
    () => {
        "<mi>&#x039E;</mi>"
    };
}

#[macro_export]
macro_rules! xi {
    () => {
        "<mi>&#x03BE;</mi>"
    };
}

#[macro_export]
macro_rules! Omicron {
    () => {
        "<mi>&#x039F;</mi>"
    };
}

#[macro_export]
macro_rules! omicron {
    () => {
        "<mi>&#x03BF;</mi>"
    };
}

#[macro_export]
macro_rules! Pi {
    () => {
        "<mi>&#x03A0;</mi>"
    };
}

#[macro_export]
macro_rules! pi {
    () => {
        "<mi>&#x03C0;</mi>"
    };
}

#[macro_export]
macro_rules! Rho {
    () => {
        "<mi>&#x03A1;</mi>"
    };
}

#[macro_export]
macro_rules! rho {
    () => {
        "<mi>&#x03C1;</mi>"
    };
}

#[macro_export]
macro_rules! Sigma {
    () => {
        "<mi>&#x03A3;</mi>"
    };
}

#[macro_export]
macro_rules! sigma {
    () => {
        "<mi>&#x03C3;</mi>"
    };
}

#[macro_export]
macro_rules! Tau {
    () => {
        "<mi>&#x03A4;</mi>"
    };
}

#[macro_export]
macro_rules! tau {
    () => {
        "<mi>&#x03C4;</mi>"
    };
}

#[macro_export]
macro_rules! Upsilon {
    () => {
        "<mi>&#x03A5;</mi>"
    };
}

#[macro_export]
macro_rules! upsilon {
    () => {
        "<mi>&#x03C5;</mi>"
    };
}

#[macro_export]
macro_rules! Phi {
    () => {
        "<mi>&#x03A6;</mi>"
    };
}

#[macro_export]
macro_rules! phi {
    () => {
        "<mi>&#x03C6;</mi>"
    };
}

#[macro_export]
macro_rules! Chi {
    () => {
        "<mi>&#x03A7;</mi>"
    };
}

#[macro_export]
macro_rules! chi {
    () => {
        "<mi>&#x03C7;</mi>"
    };
}

#[macro_export]
macro_rules! Psi {
    () => {
        "<mi>&#x03A8;</mi>"
    };
}

#[macro_export]
macro_rules! psi {
    () => {
        "<mi>&#x03C8;</mi>"
    };
}

#[macro_export]
macro_rules! Omega {
    () => {
        "<mi>&#x03A9;</mi>"
    };
}

#[macro_export]
macro_rules! omega {
    () => {
        "<mi>&#x03C9;</mi>"
    };
}

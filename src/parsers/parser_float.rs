// via: https://github.com/Geal/nom/blob/master/tests/arithmetic.rs
#[allow(unused_imports)]
use nom::{IResult, digit};

// Parser definition

use std::str;
use std::str::FromStr;

// We parse any expr_float surrounded by parens, ignoring all whitespaces around those
named!(parens<f64>, ws!(delimited!( tag!("("), expr_float, tag!(")") )) );

// We transform an integer string into a f64, ignoring surrounding whitespaces
// We look for a digit suite, and try to convert it.
// If either str::from_utf8 or FromStr::from_str fail,
// we fallback to the parens parser defined above
named!(factor <f64>, alt!(
    map_res!(
        map_res!(
            ws!(recognize!(
                alt!(
                    delimited!(digit, complete!(tag!(".")), opt!(complete!(digit))) |
                    delimited!(opt!(digit), complete!(tag!(".")), digit) |
                    digit
                )
            )),
            str::from_utf8
        ),
        FromStr::from_str
    ) | parens)
);

// We read an initial factor and for each time we find
// a * or / operator followed by another factor, we do
// the math by folding everything
named!(term <f64>, do_parse!(
    init: factor >>
    res:  fold_many0!(
        pair!(alt!(tag!("*") | tag!("/")), factor),
        init,
        |acc, (op, val): (&[u8], f64)| {
            if (op[0] as char) == '*' { (acc * val) as f64 } else { (acc / val) as f64 }
        }
    ) >>
    (res)
  )
);

named!(pub expr_float <f64>, do_parse!(
    init: term >>
    res:  fold_many0!(
        pair!(alt!(tag!("+") | tag!("-")), term),
        init,
        |acc, (op, val): (&[u8], f64)| {
            if (op[0] as char) == '+' { (acc + val) as f64 } else { (acc - val) as f64 }
        }
    ) >>
    (res)
  )
);

#[test]
fn factor_test() {
    assert_eq!(factor(&b"3"[..]), IResult::Done(&b""[..], 3.0));
    assert_eq!(factor(&b"3 "[..]), IResult::Done(&b""[..], 3.0));
    assert_eq!(factor(&b" 12"[..]), IResult::Done(&b""[..], 12.0));
    assert_eq!(factor(&b"537  "[..]), IResult::Done(&b""[..], 537.0));
    assert_eq!(factor(&b"  24   "[..]), IResult::Done(&b""[..], 24.0));
}


#[test]
fn term_test() {
    assert_eq!(term(&b" 12 *2 /  3"[..]), IResult::Done(&b""[..], 8.0));
    assert_eq!(term(&b" 2* 3  *2 *2 /  3"[..]), IResult::Done(&b""[..], 8.0));
    assert_eq!(term(&b" 48 /  3/2"[..]), IResult::Done(&b""[..], 8.0));
}

#[test]
fn expr_test() {
    assert_eq!(expr_float(&b" 1 +  2 "[..]), IResult::Done(&b""[..], 3.0));
    assert_eq!(expr_float(&b" 12 + 6 - 4+  3"[..]), IResult::Done(&b""[..], 17.0));
    assert_eq!(expr_float(&b" 1 + 2*3 + 4"[..]), IResult::Done(&b""[..], 11.0));
}

#[test]
fn parens_test() {
    assert_eq!(expr_float(&b" (  2 )"[..]), IResult::Done(&b""[..], 2.0));
    assert_eq!(expr_float(&b" 2* (  3 + 4 ) "[..]), IResult::Done(&b""[..], 14.0));
    assert_eq!(expr_float(&b"  2*2 / ( 5 - 1) + 3"[..]), IResult::Done(&b""[..], 4.0));
}

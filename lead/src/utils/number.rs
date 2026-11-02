use super::error::{Status, err};

use ordered_float::{FloatCore, OrderedFloat};

fn replace_chars(string: &str) -> String {
    let mut num = String::with_capacity(string.len() + 1);
    if string.starts_with(|c| c == 'e') {
        num.push('1');
        num.push_str(string);
    } else if string.starts_with(|c| c == '.') {
        num.push('0');
        num.push_str(string);
    } else if string.starts_with("_e") {
        num.push_str("-1");
        num.push_str(&string[1..]);
    } else if string.starts_with("_.") {
        num.push_str("-0");
        num.push_str(&string[1..]);
    } else {
        num.push_str(string);
    }
    num.replace('_', "-")
}

#[inline]
pub fn parse_number(string: &str) -> Status<i64> {
    let string = replace_chars(string);
    if string.contains('e') {
        match string.parse::<f64>() {
            Ok(n) => Ok(unsafe { n.to_int_unchecked::<i64>() }),
            _ => err!(NaN),
        }
    } else {
        Ok(string.parse::<i64>()?)
    }
}

#[inline]
pub fn parse_float(string: &str) -> Status<OrderedFloat<f64>> {
    let string = &replace_chars(string);
    if string == "-" {
        Ok(OrderedFloat::infinity())
    } else if string == "--" {
        Ok(OrderedFloat::neg_infinity())
    } else {
        Ok(OrderedFloat(replace_chars(string).parse::<f64>()?))
    }
}

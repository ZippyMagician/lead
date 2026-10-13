use super::error::{Status, err};

use ordered_float::{FloatCore, OrderedFloat};

pub fn is_number(string: &str) -> bool {
    let count_ = string.matches('_').count();
    if string.is_empty() || count_ > 2 {
        return false;
    }

    let mut expos = None;

    for (i, chr) in string.char_indices() {
        match chr {
            '_' => {
                if i > 0 && Some(i - 1) != expos {
                    return false;
                }
            }

            'e' => {
                if expos.is_some() {
                    return false;
                }

                expos = Some(i);
            }

            '0'..='9' => {}

            _ => return false,
        }
    }

    if expos.is_some() { true } else { count_ <= 1 }
}

fn replace_chars(string: &str) -> String {
    let mut num = String::with_capacity(string.len() + 1);
    if string.starts_with(|c| c == 'e' || c == '.') {
        num.push('1');
        num.push_str(string);
    } else if string.starts_with("_e") || string.starts_with("_.") {
        num.push_str("-1");
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

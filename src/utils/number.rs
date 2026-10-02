use std::num::{ParseFloatError, ParseIntError};

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
    if string.starts_with('e') {
        num.push('1');
        num.push_str(string);
    } else if string.starts_with("_e") {
        num.push_str("-1");
        num.push_str(&string[1..]);
    } else {
        num.push_str(string);
    }
    num = num.replace('_', "-");

    return num;
}

#[inline]
pub fn parse_number(string: &str) -> Result<i32, ParseIntError> {
    replace_chars(string).parse::<i32>()
}

#[inline]
pub fn parse_float(string: &str) -> Result<f32, ParseFloatError> {
    replace_chars(string).parse::<f32>()
}

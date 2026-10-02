use crate::list::List;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[repr(C)]
pub enum Scalar {
    Number(i32),
    Float(f32),
    Boolean(bool),
    Box(*const List),
}


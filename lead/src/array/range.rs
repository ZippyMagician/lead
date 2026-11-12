use crate::types::list::{Array, List};
use crate::utils::error::{Status, err};

// `List::Integers`
impl Array<i64> {
    #[inline]
    pub fn range(start: i64, end: i64, rev: bool) -> Status<List> {
        if end < start || end < 0 || start < 0 {
            return err!(Domain);
        }
        let count: usize = (end - start + 1) as usize;
        unsafe {
            let iter = start..=end;
            if rev {
                Self::alloc_iter(&[count], iter.rev())
            } else {
                Self::alloc_iter(&[count], iter)
            }
        }
    }
}

// `List::String`
impl Array<u8> {
    #[inline]
    pub fn range(start: u8, end: u8, rev: bool) -> Status<List> {
        if end < start {
            return err!(Domain);
        }
        let count: usize = (end - start + 1) as usize;
        unsafe {
            let iter = start..=end;
            if rev {
                Self::alloc_iter(&[count], iter.rev())
            } else {
                Self::alloc_iter(&[count], iter)
            }
        }
    }
}

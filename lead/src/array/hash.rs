use std::hash::{Hash, Hasher};

use super::ord::OrderedScalar;
use crate::types::list::Array;
use crate::types::scalar::{Boxed, Scalar};

use ordered_float::OrderedFloat;

pub trait HashScalar: Scalar + OrderedScalar {
    fn hash_scalar<H: Hasher>(&self, state: &mut H);
}

impl<T: HashScalar> Hash for Array<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.rank().hash(state);
        for s in self.list() {
            s.hash_scalar(state);
        }
    }
}

// -> f64 -> bits (u64) if hashing against each other
impl HashScalar for i64 {
    fn hash_scalar<H: Hasher>(&self, s: &mut H) {
        self.hash(s)
    }
}

impl HashScalar for u8 {
    fn hash_scalar<H: Hasher>(&self, s: &mut H) {
        self.hash(s)
    }
}

impl HashScalar for bool {
    fn hash_scalar<H: Hasher>(&self, s: &mut H) {
        self.hash(s)
    }
}

impl HashScalar for OrderedFloat<f64> {
    fn hash_scalar<H: Hasher>(&self, s: &mut H) {
        self.hash(s)
    }
}

impl HashScalar for Boxed {
    fn hash_scalar<H: Hasher>(&self, s: &mut H) {
        // Hash by list or address
        // FIXME: This matches assumptions in `array::ord`, but is kinda iffy in general
        if let crate::types::Type::List(l) = self.unbox() {
            l.hash(s);
        } else {
            self.addr().hash(s);
        }
    }
}

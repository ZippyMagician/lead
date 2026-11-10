use std::rc::Rc;

use crate::types::list::{Array, List};
use crate::types::scalar::{Boxed, Scalar};
use crate::utils::error::{Status, err};

use ordered_float::OrderedFloat;

pub trait Castable {
    fn cast_i64(self) -> Status<Array<i64>>;
    fn cast_f64(self) -> Status<Array<OrderedFloat<f64>>>;
    fn cast_bool(self) -> Status<Array<bool>>;
    fn cast_char(self) -> Status<Array<u8>>;
}

impl<T: Scalar> Array<T>
where
    Array<T>: Castable,
{
    #[inline]
    pub fn to_cast(self, tag: u8) -> Status<List> {
        let res = match tag {
            b'I' => match self {
                _ if T::get_type() == tag => T::wrap(self),
                _ => List::Integers(self.cast_i64()?),
            },
            b'F' => match self {
                _ if T::get_type() == tag => T::wrap(self),
                _ => List::Doubles(self.cast_f64()?),
            },
            b'C' => match self {
                _ if T::get_type() == tag => T::wrap(self),
                _ => List::String(self.cast_char()?),
            },
            b'B' => match self {
                _ if T::get_type() == tag => T::wrap(self),
                _ => List::Booleans(self.cast_bool()?),
            },
            _ => {
                std::hint::cold_path();
                return err!(Value);
            }
        };
        Ok(res)
    }
    
    #[inline(always)]
    pub unsafe fn treat_as<U: Scalar>(self) -> Array<U> {
        unsafe {
            std::mem::transmute(self)
        }
    }

    #[inline(always)]
    pub unsafe fn treat_as_ref<U: Scalar>(&self) -> &Array<U> {
        unsafe {
            std::mem::transmute(self)
        }
    }
}

impl Castable for Array<i64> {
    #[inline(always)]
    fn cast_i64(self) -> Status<Self> {
        Ok(self)
    }

    #[inline]
    fn cast_f64(mut self) -> Status<Array<OrderedFloat<f64>>> {
        if Rc::get_mut(&mut self.dt.v).is_none() {
            let rc: Rc<[OrderedFloat<f64>]> =
                self.list().iter().map(|n| n.to_float().unwrap()).collect();
            Ok(Array::new(rc, self.ds, self.rk))
        } else {
            let vals = Rc::get_mut(&mut self.dt.v).unwrap();
            // SAFETY: trust me bro
            // i64 and f64 are the same size, so we do some transmutation fuckery to perform the
            // casting in place
            unsafe {
                let floats: &mut [OrderedFloat<f64>] = std::mem::transmute(vals);
                for i in 0..floats.len() {
                    floats[i] = std::mem::transmute::<OrderedFloat<f64>, i64>(floats[i])
                        .to_float()
                        .unwrap();
                }
                Ok(Array::new(
                    std::mem::transmute::<_, Rc<[OrderedFloat<f64>]>>(self.dt.v),
                    self.ds,
                    self.rk,
                ))
            }
        }
    }

    #[inline]
    fn cast_bool(self) -> Status<Array<bool>> {
        let rc: Rc<[bool]> = self.list().iter().map(|n| n.to_bool().unwrap()).collect();
        Ok(Array::new(rc, self.ds, self.rk))
    }

    #[inline]
    fn cast_char(self) -> Status<Array<u8>> {
        let rc: Rc<[u8]> = self.list().iter().map(|n| n.to_char()).collect::<Status<_>>()?;
        Ok(Array::new(rc, self.ds, self.rk))
    }
}

impl Castable for Array<OrderedFloat<f64>> {
    #[inline(always)]
    fn cast_f64(self) -> Status<Self> {
        Ok(self)
    }

    #[inline]
    fn cast_i64(mut self) -> Status<Array<i64>> {
        if Rc::get_mut(&mut self.dt.v).is_none() {
            let rc: Rc<[i64]> = self.list().iter().map(|n| n.to_num()).collect::<Status<_>>()?;
            Ok(Array::new(rc, self.ds, self.rk))
        } else {
            let vals = Rc::get_mut(&mut self.dt.v).unwrap();
            // SAFETY: see above
            unsafe {
                let nums: &mut [i64] = std::mem::transmute(vals);
                for i in 0..nums.len() {
                    nums[i] = std::mem::transmute::<i64, OrderedFloat<f64>>(nums[i]).to_num()?;
                }
                Ok(Array::new(std::mem::transmute::<_, Rc<[i64]>>(self.dt.v), self.ds, self.rk))
            }
        }
    }

    #[inline]
    fn cast_bool(self) -> Status<Array<bool>> {
        let rc: Rc<[bool]> = self.list().iter().map(|n| n.to_bool().unwrap()).collect();
        Ok(Array::new(rc, self.ds, self.rk))
    }

    #[inline]
    fn cast_char(self) -> Status<Array<u8>> {
        let rc: Rc<[u8]> = self.list().iter().map(|n| n.to_char()).collect::<Status<_>>()?;
        Ok(Array::new(rc, self.ds, self.rk))
    }
}

impl Castable for Array<bool> {
    #[inline(always)]
    fn cast_bool(self) -> Status<Self> {
        Ok(self)
    }

    #[inline]
    fn cast_i64(self) -> Status<Array<i64>> {
        let rc: Rc<[i64]> = self.list().iter().map(|n| n.to_num().unwrap()).collect();
        Ok(Array::new(rc, self.ds, self.rk))
    }

    #[inline]
    fn cast_f64(self) -> Status<Array<OrderedFloat<f64>>> {
        let rc: Rc<[OrderedFloat<f64>]> =
            self.list().iter().map(|n| n.to_float().unwrap()).collect();
        Ok(Array::new(rc, self.ds, self.rk))
    }

    // This could be in place conditionally but seriously, when are you ever going to be casting a
    // list of booleans to a list of characters
    #[inline]
    fn cast_char(self) -> Status<Array<u8>> {
        let rc: Rc<[u8]> = self.list().iter().map(|n| n.to_char().unwrap()).collect();
        Ok(Array::new(rc, self.ds, self.rk))
    }
}

impl Castable for Array<u8> {
    #[inline(always)]
    fn cast_char(self) -> Status<Self> {
        Ok(self)
    }

    #[inline]
    fn cast_i64(self) -> Status<Array<i64>> {
        let rc: Rc<[i64]> = self.list().iter().map(|n| n.to_num().unwrap()).collect();
        Ok(Array::new(rc, self.ds, self.rk))
    }

    #[inline]
    fn cast_f64(self) -> Status<Array<OrderedFloat<f64>>> {
        let rc: Rc<[OrderedFloat<f64>]> =
            self.list().iter().map(|n| n.to_float().unwrap()).collect();
        Ok(Array::new(rc, self.ds, self.rk))
    }

    #[inline]
    fn cast_bool(self) -> Status<Array<bool>> {
        let rc: Rc<[bool]> = self.list().iter().map(|n| n.to_bool().unwrap()).collect();
        Ok(Array::new(rc, self.ds, self.rk))
    }
}

impl Castable for Array<Boxed> {
    #[inline(always)]
    fn cast_i64(self) -> Status<Array<i64>> {
        err!(Box)
    }

    #[inline(always)]
    fn cast_f64(self) -> Status<Array<OrderedFloat<f64>>> {
        err!(Box)
    }

    #[inline(always)]
    fn cast_bool(self) -> Status<Array<bool>> {
        err!(Box)
    }

    #[inline(always)]
    fn cast_char(self) -> Status<Array<u8>> {
        err!(Box)
    }
}

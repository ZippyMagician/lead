#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Scalar {
    Number(i32),
    Float(f32),
    Boolean(bool),
    Char(char),
    Box(*const List),
}

pub struct List {
    inner: Vec<Scalar>,
    rank: Vec<usize>,
}

impl List {
    pub fn new() -> Self {
        Self { inner: vec![], rank: vec![0] }
    }

    #[inline(always)]
    pub fn dims(&self) -> usize {
        self.rank.len()
    }

    #[inline(always)]
    pub fn as_list(&self) -> &[Scalar] {
        &self.inner
    }

    #[inline(always)]
    pub fn get_rank(&self) -> &[usize] {
        &self.rank
    }
}

use std::fmt::Display;
use std::num::NonZeroU128;
use std::ops::{Add, Sub};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Row(NonZeroU128);

impl Display for Row {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl Add<u128> for Row {
    type Output = Option<Row>;

    fn add(self, rhs: u128) -> Self::Output {
        self.0.checked_add(rhs).map(Row)
    }
}

impl Sub<u128> for Row {
    type Output = Option<Row>;

    fn sub(self, rhs: u128) -> Self::Output {
        self.0
            .get()
            .checked_sub(rhs)
            .and_then(NonZeroU128::new)
            .map(Row)
    }
}

impl Row {
    pub const FIRST: Self = Self(NonZeroU128::MIN);
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Col(u128);

impl Add<u128> for Col {
    type Output = Option<Col>;

    fn add(self, rhs: u128) -> Self::Output {
        self.0.checked_add(rhs).map(Col)
    }
}

impl Sub<u128> for Col {
    type Output = Option<Col>;

    fn sub(self, rhs: u128) -> Self::Output {
        self.0.checked_sub(rhs).map(Col)
    }
}

impl Col {
    pub const FIRST: Self = Self(0);
}

impl Display for Col {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        let mut col = self.0;
        loop {
            let rem = (col % 26) as u8;
            col /= 26;
            s.insert(0, (b'A' + rem).into());
            if col == 0 {
                return f.write_str(&s);
            }
        }
    }
}

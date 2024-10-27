use std::fmt::Display;
use std::num::NonZeroU128;
use std::ops::{Add, Sub};
use std::str::FromStr;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Row(NonZeroU128);

impl Display for Row {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for Row {
    type Err = <NonZeroU128 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        NonZeroU128::from_str(s).map(Self)
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
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

#[derive(PartialEq, Debug)]
pub struct ParseColError;

impl FromStr for Col {
    type Err = ParseColError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut col = 0_u128;
        for c in s.bytes() {
            match c {
                b'A'..=b'Z' => {
                    col *= u128::from(b'Z' - b'A' + 1);
                    col += u128::from(c - b'A' + 1);
                }
                _ => return Err(ParseColError),
            }
        }
        Ok(Self(col - 1))
    }
}

#[test]
fn col_roundtrip() {
    assert_eq!("A".parse(), Ok(Col(0)));
    assert_eq!("B".parse(), Ok(Col(1)));
    assert_eq!("Z".parse(), Ok(Col(25)));
    assert_eq!(Col::FIRST.to_string().parse(), Ok(Col::FIRST));
    assert_eq!("AA".parse(), Ok(Col(26)));
    assert_eq!("DD".parse(), Ok(Col(26 * 4 + 3)));
}

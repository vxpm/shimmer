use std::ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign};

/// A signed integer with `N` total bits.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Integer<const N: usize> {
    /// The actual value.
    value: i64,
    /// Has any operation overflowed the value?
    overflow: bool,
    /// Has any operation underflowed the value?
    underflow: bool,
}

impl<const N: usize> std::fmt::Debug for Integer<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}

impl<const N: usize> Integer<N> {
    pub const MAX: i64 = (1 << (N - 1));
    pub const MIN: i64 = -Self::MAX;

    #[inline(always)]
    fn ensure_sign(self) -> Self {
        Self {
            value: (self.value << (64 - N)) >> (64 - N),
            ..self
        }
    }

    #[inline(always)]
    pub fn new(value: i64) -> Self {
        Self {
            value,
            overflow: false,
            underflow: false,
        }
        .ensure_sign()
    }

    #[inline(always)]
    pub fn value(&self) -> i64 {
        self.value
    }

    #[inline(always)]
    pub fn overflowed(&self) -> bool {
        self.overflow
    }

    #[inline(always)]
    pub fn underflowed(&self) -> bool {
        self.underflow
    }

    #[inline(always)]
    pub fn clean(&self) -> Self {
        Self {
            value: self.value,
            overflow: false,
            underflow: false,
        }
    }
}

impl<const N: usize> Add for Integer<N> {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        let sum = self.value + rhs.value;
        let overflow = sum > Self::MAX;
        let underflow = sum < Self::MIN;

        Self {
            value: sum,
            overflow: self.overflow | rhs.overflow | overflow,
            underflow: self.underflow | rhs.underflow | underflow,
        }
        .ensure_sign()
    }
}

impl<const N: usize> AddAssign for Integer<N> {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl<const N: usize> Sub for Integer<N> {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        let sum = self.value - rhs.value;
        let overflow = sum > Self::MAX;
        let underflow = sum < Self::MIN;

        Self {
            value: sum,
            overflow: self.overflow | rhs.overflow | overflow,
            underflow: self.underflow | rhs.underflow | underflow,
        }
        .ensure_sign()
    }
}

impl<const N: usize> SubAssign for Integer<N> {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl<const N: usize> Mul for Integer<N> {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        let prod = self.value * rhs.value;

        Self {
            value: prod,
            overflow: self.overflow | rhs.overflow,
            underflow: self.underflow | rhs.underflow,
        }
        .ensure_sign()
    }
}

impl<const N: usize> MulAssign for Integer<N> {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

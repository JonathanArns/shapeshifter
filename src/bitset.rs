use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl, ShlAssign, Shr, ShrAssign};

/// An efficient, compiletime sized Bitset
///
/// Bits outside of the size of the bitset might are undefined, but can be accessed.
/// They might have any value.
///
/// A lot of the implementation is based on the rust-dense-bitset crate.
#[derive(Clone, Copy)]
pub struct Bitset<const N: usize> {
    state: [u128; N],
}

impl<const N: usize> Bitset<N> {
    pub fn new() -> Self {
        Bitset{
            state: [0; N]
        }
    }

    /// Returns a new Bitset with only the specifies bit set to true
    pub fn with_bit_set(idx: usize) -> Self {
        let mut ret = Self::new();
        ret.set_bit(idx);
        ret
    }

    /// Sets the bit at index position to true
    pub fn set_bit(&mut self, position: usize) {
        let i = position >> 7;
        let offset = position % 128;
        self.state[i] |= 1_u128<<offset;
    }
    
    /// Sets the bit at index position to false
    pub fn unset_bit(&mut self, position: usize) {
        let i = position >> 7;
        let offset = position % 128;
        self.state[i] &= !(1_u128<<offset);
    }
    
    /// Get the bit at index position
    /// Panics if position is out of range
    pub fn get_bit(&self, position: usize) -> bool {
        let i = position >> 7;
        let offset = position % 128;
        (self.state[i]>>offset) & 1 == 1
    }

    /// Returns `true` if at least one bit is set to `true`
    pub fn any(&self) -> bool {
        for x in self.state {
            if x != 0 {
                return true
            }
        }
        false
    }

    /// Returns `true` if all the bits are set to `false`
    pub fn none(&self) -> bool {
        !self.any()
    }

    /// Returns the number of ones in the Bitset
    pub fn count_ones(&self) -> u32 {
        let mut ones = 0;
        for x in self.state {
            ones += x.count_ones();
        }
        ones
    }

    /// Returns the number of zeros in the Bitset
    pub fn count_zeros(&self) -> u32 {
        let mut zeros = 0;
        for x in self.state {
            zeros += x.count_zeros();
        }
        zeros
    }
}

impl<const N: usize> std::fmt::Debug for Bitset<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut result = String::new();
        for i in 0..self.state.len() {
            result += "_";
            for j in 0..128 {
                result += if self.get_bit((self.state.len() - i - 1) * 128 + (127 - j)) {
                    "1"
                } else {
                    "0"
                };
            }
        }
        write!(f, "0b{}", result)
    }
}

impl<const N: usize> Not for Bitset<N> {
    type Output = Self;
    fn not(self) -> Self {
        let mut ret = Self::new();
        for i in 0..self.state.len() {
            ret.state[i] = !self.state[i];
        }
        ret
    }
}

impl<const N: usize> BitAnd for Bitset<N> {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        let mut ret = Self::new();
        for i in 0..self.state.len() {
            ret.state[i] = self.state[i] & rhs.state[i];
        }
        ret
    }
}

impl<const N: usize> BitAndAssign for Bitset<N> {
    fn bitand_assign(&mut self, rhs: Self) {
        for i in 0..self.state.len() {
            self.state[i] &= rhs.state[i];
        }
    }
}

impl<const N: usize> BitOr for Bitset<N> {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        let mut ret = Self::new();
        for i in 0..self.state.len() {
            ret.state[i] = self.state[i] | rhs.state[i];
        }
        ret
    }
}

impl<const N: usize> BitOrAssign for Bitset<N> {
    fn bitor_assign(&mut self, rhs: Self) {
        for i in 0..self.state.len() {
            self.state[i] |= rhs.state[i];
        }
    }
}

impl<const N: usize> BitXor for Bitset<N> {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self {
        let mut ret = Self::new();
        for i in 0..self.state.len() {
            ret.state[i] = self.state[i] ^ rhs.state[i];
        }
        ret
    }
}

impl<const N: usize> BitXorAssign for Bitset<N> {
    fn bitxor_assign(&mut self, rhs: Self) {
        for i in 0..self.state.len() {
            self.state[i] ^= rhs.state[i];
        }
    }
}

impl<const N: usize> Shl<usize> for Bitset<N> {
    type Output = Self;
    fn shl(self, rhs: usize) -> Self {
        let mut ret = Self::new();
        let trailing_zeros = rhs >> 7;
        let actual_shift = rhs % 128;
        let l = self.state.len();
        if trailing_zeros >= l {
            return ret
        }
        if actual_shift == 0 {
            for i in 0..(l-trailing_zeros) {
                ret.state[l-i-1] = self.state[l-i-1-trailing_zeros];
            }
        } else {
            for i in 0..(l-1-trailing_zeros) {
                ret.state[l-i-1] = (self.state[l-i-1-trailing_zeros]<<actual_shift) | (self.state[l-i-2-trailing_zeros]>>(128-actual_shift));
            }
            ret.state[trailing_zeros] = self.state[0]<<actual_shift;
        }
        ret
    }
}

impl<const N: usize> ShlAssign<usize> for Bitset<N> {
    fn shl_assign(&mut self, rhs: usize) {
        let trailing_zeros = rhs >> 7;
        let actual_shift = rhs % 128;
        let l = self.state.len();
        if actual_shift == 0 {
            for i in 0..(l-trailing_zeros) {
                self.state[l-i-1] = self.state[l-i-1-trailing_zeros];
            }
        } else {
            for i in 0..(l-1-trailing_zeros) {
                self.state[l-i-1] = (self.state[l-i-1-trailing_zeros]<<actual_shift) | (self.state[l-i-2-trailing_zeros]>>(128-actual_shift));
            }
            self.state[trailing_zeros] = self.state[0]<<actual_shift;
        }
        for i in 0..trailing_zeros {
            self.state[i] = 0;
        }
    }
}

impl<const N: usize> Shr<usize> for Bitset<N> {
    type Output = Self;
    fn shr(self, rhs: usize) -> Self {
        let mut ret = Self::new();
        let leading_zeros = rhs >> 7;
        let actual_shift = rhs % 128;
        let l = self.state.len();
        if leading_zeros >= l {
            return ret
        }
        if actual_shift == 0 {
            for i in 0..(l-leading_zeros) {
                ret.state[i] = self.state[i+leading_zeros];
            }
        } else {
            for i in 0..(l-1-leading_zeros) {
                ret.state[i] = (self.state[i+leading_zeros]>>actual_shift) | (self.state[i+1+leading_zeros]<<(128-actual_shift));
            }
        }
        ret
    }
}

impl<const N: usize> ShrAssign<usize> for Bitset<N> {
    fn shr_assign(&mut self, rhs: usize) {
        let leading_zeros = rhs >> 7;
        let actual_shift = rhs % 128;
        let l = self.state.len();
        if actual_shift == 0 {
            for i in 0..(l-leading_zeros) {
                self.state[i] = self.state[i+leading_zeros];
            }
        } else {
            for i in 0..(l-1-leading_zeros) {
                self.state[i] = (self.state[i+leading_zeros]>>actual_shift) | (self.state[i+1+leading_zeros]<<(128-actual_shift));
            }
            if leading_zeros < l {
                self.state[0] = self.state[leading_zeros]>>actual_shift;
            }
        }
        for i in 0..leading_zeros {
            self.state[l-1-i] = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    #[test]
    fn test_shl() {
        let x = Bitset::<2>{state: [1, 0]};
        println!("{:?}", x<<129);
        assert!((x<<129).state[0] == 0);
        assert!((x<<129).state[1] == 2);
    }

    #[test]
    fn test_shl_or() {
        let x = Bitset::<2>{state: [1, 0]};
        let mut y = Bitset::<2>{state: [0, 0]};
        for i in 0..256 {
            y |= x << i;
        }
        println!("{:?}", y);
        assert!(y.state[0] == u128::MAX);
        assert!(y.state[1] == u128::MAX);
    }

    #[test]
    fn test_shl_assign() {
        let mut x = Bitset::<2>{state: [1, 0]};
        x <<= 129;
        println!("{:?}", x);
        assert!(x.state[0] == 0);
        assert!(x.state[1] == 2);
    }

    #[test]
    fn test_shl_assign_or() {
        let mut x = Bitset::<2>{state: [1, 0]};
        let mut y = Bitset::<2>{state: [1, 0]};
        for _ in 0..256 {
            x <<= 1;
            y |= x;
        }
        println!("{:?}", y);
        assert!(y.state[0] == u128::MAX);
        assert!(y.state[1] == u128::MAX);
    }

    #[test]
    fn test_shr_or() {
        let x = Bitset::<2>{state: [0, 1]};
        let mut y = Bitset::<2>{state: [0, 0]};
        for i in 0..256 {
            y |= x >> i;
        }
        println!("{:?}", y);
        assert!(y.state[0] == u128::MAX);
        assert!(y.state[1] == 1);
    }

    #[test]
    fn test_shl_assign_shr_assign() {
        let mut x = Bitset::<2>{state: [1, 0]};
        x <<= 130;
        x >>= 129;
        println!("{:?}", x);
        assert!(x.state[0] == 2);
        assert!(x.state[1] == 0);
    }

    #[bench]
    fn bench_shl_or_bitset_8(b: &mut Bencher) {
        let x = Bitset::<8>{state: [1, 0, 0, 0, 0, 0, 0, 0]};
        let mut y = Bitset::<8>{state: [0, 0, 0, 0, 0, 0, 0, 0]};
        b.iter(|| {
            for i in 0..128 {
                y |= x << i;
            }
            y
        })
    }

    #[bench]
    fn bench_shl_or_bitset_1(b: &mut Bencher) {
        let x = Bitset::<1>{state: [1]};
        let mut y = Bitset::<1>{state: [0]};
        b.iter(|| {
            for i in 0..128 {
                y |= x << i;
            }
            y
        })
    }

    #[bench]
    fn bench_shl_or_u128(b: &mut Bencher) {
        let x = 1_u128;
        let mut y = 0_u128;
        b.iter(|| {
            for i in 0..128 {
                y |= x << i;
            }
            y
        })
    }
}

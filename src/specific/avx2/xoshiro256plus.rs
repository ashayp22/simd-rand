use std::{
    arch::x86_64::*,
    mem::{self, transmute},
    ops::{Deref, DerefMut},
};

use rand_core::SeedableRng;

use crate::specific::avx2::read_u64_into_vec;

use super::{simdprng::*, rotate_left};
use super::vecs::*;

pub struct Xoshiro256PlusX4Seed([u8; 128]);

impl Xoshiro256PlusX4Seed {
    pub fn new(seed: [u8; 128]) -> Self {
        Self(seed)
    }
}

impl Into<Xoshiro256PlusX4Seed> for [u8; 128] {
    fn into(self) -> Xoshiro256PlusX4Seed {
        Xoshiro256PlusX4Seed::new(self)
    }
}

impl Into<Xoshiro256PlusX4Seed> for Vec<u8> {
    fn into(self) -> Xoshiro256PlusX4Seed {
        assert!(self.len() == 128);
        Xoshiro256PlusX4Seed::new(self.try_into().unwrap())
    }
}

impl Deref for Xoshiro256PlusX4Seed {
    type Target = [u8; 128];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Xoshiro256PlusX4Seed {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[repr(align(32))]
pub struct Xoshiro256PlusX4 {
    s0: __m256i,
    s1: __m256i,
    s2: __m256i,
    s3: __m256i,
}
impl Default for Xoshiro256PlusX4Seed {
    fn default() -> Xoshiro256PlusX4Seed {
        Xoshiro256PlusX4Seed([0; 128])
    }
}

impl AsMut<[u8]> for Xoshiro256PlusX4Seed {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl SeedableRng for Xoshiro256PlusX4 {
    type Seed = Xoshiro256PlusX4Seed;

    fn from_seed(seed: Self::Seed) -> Self {
        const SIZE: usize = mem::size_of::<u64>();
        const LEN: usize = 4;
        const VECSIZE: usize = SIZE * LEN;
        unsafe {
            let mut s0: __m256i = _mm256_setzero_si256();
            let mut s1: __m256i = _mm256_setzero_si256();
            let mut s2: __m256i = _mm256_setzero_si256();
            let mut s3: __m256i = _mm256_setzero_si256();
            read_u64_into_vec(&seed[(VECSIZE * 0)..(VECSIZE * 1)], &mut s0);
            read_u64_into_vec(&seed[(VECSIZE * 1)..(VECSIZE * 2)], &mut s1);
            read_u64_into_vec(&seed[(VECSIZE * 2)..(VECSIZE * 3)], &mut s2);
            read_u64_into_vec(&seed[(VECSIZE * 3)..(VECSIZE * 4)], &mut s3);

            Self { s0, s1, s2, s3 }
        }
    }
}

impl SimdPrng for Xoshiro256PlusX4 {
    #[inline(always)]
    fn next_m256i(&mut self, vector: &mut __m256i) {
        unsafe {
            // const uint64_t result = s[0] + s[3];
            *vector = _mm256_add_epi64(self.s0, self.s3);

            // const uint64_t t = s[1] << 17;
            let t = _mm256_sll_epi64(self.s1, _mm_cvtsi32_si128(17));
        
            // s[2] ^= s[0];
            // s[3] ^= s[1];
            // s[1] ^= s[2];
            // s[0] ^= s[3];
            self.s2 = _mm256_xor_si256(self.s2, self.s0);
            self.s3 = _mm256_xor_si256(self.s3, self.s1);
            self.s1 = _mm256_xor_si256(self.s1, self.s2);
            self.s0 = _mm256_xor_si256(self.s0, self.s3);
        
            // s[2] ^= t;
            self.s2 = _mm256_xor_si256(self.s2, t);
        
            // s[3] = rotl(s[3], 45);
            self.s3 = rotate_left::<45>(self.s3);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem;

    use itertools::Itertools;
    use num_traits::PrimInt;
    use rand_core::{SeedableRng, RngCore};
    use serial_test::parallel;

    use crate::testutil::{test_uniform_distribution, DOUBLE_RANGE, REF_SEED_256};

    use super::*;

    #[test]
    #[parallel]
    fn reference() {
        let seed: Xoshiro256PlusX4Seed = REF_SEED_256.into();
        let mut rng = Xoshiro256PlusX4::from_seed(seed);
        // These values were produced with the reference implementation:
        // http://xoshiro.di.unimi.it/xoshiro256plusplus.c
        #[rustfmt::skip]
        let expected = [
            5,
            211106232532999,
            211106635186183,
            9223759065350669058,
            9250833439874351877,
            13862484359527728515,
            2346507365006083650,
            1168864526675804870,
            34095955243042024,
            3466914240207415127,
        ];
        for &e in &expected {
            let mut mem = Default::default();
            rng.next_u64x4(&mut mem);
            for v in mem.into_iter() {
                assert_eq!(v, e);
            }
        }
    }
}
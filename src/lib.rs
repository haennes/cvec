#![allow(incomplete_features)]
#![feature(
    const_mut_refs,
    generic_const_exprs,
    slice_as_chunks,
    const_option,
    const_refs_to_cell,
    adt_const_params
)]

use core::marker::ConstParamTy;

use coption::COption;
#[cfg(feature = "self_rust_tokenize")]
use quote::quote;
#[cfg(feature = "self_rust_tokenize")]
use self_rust_tokenize::{QuoteToTokens, SelfRustTokenize};

#[derive(Eq, PartialEq, ConstParamTy)]
pub struct CVec<T: Copy, const LEN: usize>([COption<T>; LEN]);

macro_rules! cvec {
    ($len: expr, $($elem: expr),*) => {
        {

            let mut temp_vec: CVec<_, $len> = CVec::empty();
            $(
                temp_vec.insert($elem);
            )*
            temp_vec
        }
    };
}

impl<T: Copy + Eq, const LEN: usize> CVec<T, LEN>
where
    [(); LEN - 1]:,
{
    pub const fn new_slice(slice: &[T]) -> Self {
        let mut arr = [COption::None; LEN];
        let mut idx = 0;
        while idx < LEN && idx < slice.len() {
            arr[idx] = COption::Some(slice[idx]);
            idx += 1;
        }
        Self(arr)
    }

    pub const fn new_arr(arr: [T; LEN]) -> Self {
        let mut opt_arr = [COption::None; LEN];
        let mut idx = 0;
        while idx < LEN {
            opt_arr[idx] = COption::Some(arr[idx]);
            idx += 1;
        }
        Self(opt_arr)
    }
    pub const fn empty() -> Self {
        Self([COption::None; LEN])
    }

    pub const fn get(&self, idx: usize) -> Option<T> {
        self.0[idx].into_std()
    }

    pub const fn poped(&self) -> (Option<T>, CVec<T, LEN>) {
        let slice = self.0.as_slice();
        let (first, other) = slice.split_first().unwrap();
        let other = &other.as_chunks::<{ LEN - 1 }>().0[0];
        let mut idx = 0;
        let mut new_inner = [COption::None; LEN];
        while idx < LEN {
            new_inner[idx] = other[idx];
            idx += 1;
        }
        // no need to compress. Is already continuous if input was continuos
        (first.into_std(), CVec(new_inner))
    }

    pub const fn extended_one(&self) -> CVec<T, { LEN + 1 }> {
        let inner = self.0;
        let mut idx = 0;
        let mut new_inner = [COption::None; { LEN + 1 }];
        while idx < LEN {
            new_inner[idx] = inner[idx];
            idx += 1;
        }
        CVec(new_inner)
    }

    pub const fn insert(&mut self, item: T) {
        let mut idx = 0;
        while idx < LEN {
            if self.0[idx].into_std().is_none() {
                self.0[idx] = COption::Some(item);
                return;
            }
            idx += 1;
        }
        panic!("failed to insert. already full")
    }

    pub const fn remove(&mut self, idx: usize) -> T {
        let to_return = self.remove_not_compressed(idx);
        self.compress();
        to_return
    }

    pub const fn remove_not_compressed(&mut self, idx: usize) -> T {
        let to_return = self.0[idx];
        self.0[idx] = COption::None;
        to_return.into_std().expect("no element present at idx")
    }

    pub const fn compress(&mut self) {
        loop {
            if self.compress_once() {
                // try until we do not need to compress any more
                break;
            }
        }
    }

    ///returns wether it was already continuos
    const fn compress_once(&mut self) -> bool {
        let idx = 0;
        let mut continuos: bool = true;
        while idx < LEN - 1 {
            if continuos {
                if self.0[idx].into_std().is_none() {
                    continuos = false;
                }
            } else {
                self.0[idx] = self.0[idx + 1]
            }
        }
        // we DIDNT have to compress
        continuos
    }
}

#[cfg(feature = "self_rust_tokenize")]
impl<T: Copy + QuoteToTokens, const LEN: usize> SelfRustTokenize for CVec<T, LEN> {
    fn append_to_token_stream(
        &self,
        token_stream: &mut self_rust_tokenize::proc_macro2::TokenStream,
    ) {
        let t = quote!(T);
        let len = quote!(LEN);
        let items = self.0.iter().map(|opt| {
            let opt = &opt.to_tokens();
            quote!(#opt)
        });

        let q = quote!(
            CVec::<#t, #len>::new_arr([#(#items),*])
        );
        token_stream.extend(q)
    }
}

#[cfg(test)]
mod tests {
    use super::CVec;

    #[cfg(feature = "self_rust_tokenize")]
    use super::quote;
    #[cfg(feature = "self_rust_tokenize")]
    use self_rust_tokenize::SelfRustTokenize;

    #[cfg(feature = "self_rust_tokenize")]
    #[test]
    fn quote() {
        let mut vec: CVec<u8, 10> = CVec::empty();
        vec.insert(8);
        let vec_ts = vec.to_tokens();
        let expected_ts = quote!(CVec::<T, LEN>::new_arr([
            COption::Some(8u8),
            COption::None,
            COption::None,
            COption::None,
            COption::None,
            COption::None,
            COption::None,
            COption::None,
            COption::None,
            COption::None
        ]));
        let vec_str = format!("{}", vec_ts);
        let expected_str = format!("{}", expected_ts);
        assert_eq!(vec_str, expected_str);
    }

    #[cfg(feature = "self_rust_tokenize")]
    #[test]
    fn cvec_macro() {
        let vec_ts = cvec!(10, 1u16, 4u16).to_tokens();
        let expected_ts = quote!(CVec::<T, LEN>::new_arr([
            COption::Some(1u16),
            COption::Some(4u16),
            COption::None,
            COption::None,
            COption::None,
            COption::None,
            COption::None,
            COption::None,
            COption::None,
            COption::None
        ]));
        assert_eq!(format!("{}", vec_ts), format!("{}", expected_ts));
    }
}

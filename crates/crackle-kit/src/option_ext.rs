/// # Example
/// ```rust
/// impl_option_handle_trait!(SliceOption, ok_or_slice_err, anyhow!("slice failed"));
///
/// // will expand to (default visibility: pub(crate)):
/// pub(crate) trait SliceOption<T> {
///     fn ok_or_slice_err(self) -> Result<T, Error>;
/// }
///
/// impl<I> SliceOption<I> for Option<I>
/// {
///     fn ok_or_slice_err(self) -> Result<I, Error> {
///         match self {
///             Some(v) => Ok(v),
///             None => Err(anyhow!("Slice failed")),
///         }
///     }
/// }
///
/// // or you can set visibility:
/// impl_option_handle_trait!(pub, SliceOption, ok_or_slice_err, anyhow!("slice failed"));
/// ```
macro_rules! impl_option_handle_trait {
    ($trait_name:ident, $method_name:ident, $err_expr:expr) => {
        $crate::impl_option_handle_trait!(pub(crate), $trait_name, $method_name, $err_expr);
    };
    ($vis:vis, $trait_name:ident, $method_name:ident, $err_expr:expr) => {
        $vis trait $trait_name<T> {
            fn $method_name(self) -> Result<T, anyhow::Error>;
        }

        impl<T> $trait_name<T> for Option<T> {
            fn $method_name(self) -> Result<T, anyhow::Error> {
                match self {
                    Some(v) => Ok(v),
                    None => Err($err_expr)?,
                }
            }
        }
    };
}

use std::{borrow::Borrow, collections::HashMap, hash::Hash};

use anyhow::Error;
pub(crate) use impl_option_handle_trait;

pub(crate) trait HashMapExt<K, V> {
    fn get_or_keyerr<Q>(&self, k: &Q) -> Result<&V, Error>
    where
        K: Borrow<Q>,
        Q: std::fmt::Display + Hash + Eq + ?Sized;

    fn get_mut_or_keyerr<Q>(&mut self, k: &Q) -> Result<&mut V, Error>
    where
        K: Borrow<Q>,
        Q: std::fmt::Display + Hash + Eq + ?Sized;
}

impl<K, V> HashMapExt<K, V> for HashMap<K, V>
where
    K: Hash + Eq,
{
    fn get_or_keyerr<Q>(&self, k: &Q) -> Result<&V, Error>
    where
        K: Borrow<Q>,
        Q: std::fmt::Display + Hash + Eq + ?Sized,
    {
        match self.get(k) {
            Some(v) => Ok(v),
            None => Err(anyhow::anyhow!("Key {} not found", k)),
        }
    }

    fn get_mut_or_keyerr<Q>(&mut self, k: &Q) -> Result<&mut V, Error>
    where
        K: Borrow<Q>,
        Q: std::fmt::Display + Hash + Eq + ?Sized,
    {
        match self.get_mut(k) {
            Some(v) => Ok(v),
            None => Err(anyhow::anyhow!("Key {} not found", k)),
        }
    }
}

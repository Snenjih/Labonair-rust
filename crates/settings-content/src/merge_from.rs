//! [`MergeFrom`] — recursive settings-layer merge (default < user < project).
//!
//! Trimmed port of `zed-refrence/zed/crates/settings_content/src/merge_from.rs`,
//! cut down to the primitive/collection impls this tree actually needs (no
//! `HashMap`/`IndexMap`/`Arc` — this tree only uses `BTreeMap` for
//! deterministic serialization).

use std::collections::BTreeMap;

/// Merge `other` into `self`. The default behaviour is:
/// * `Option<T>`: `None` on `other` is ignored; `Some` recurses (or replaces
///   if `self` was `None`).
/// * `BTreeMap<K, V>`: merged key-wise, recursing per value.
/// * Everything else (including `Vec<T>` and primitives): `other` replaces
///   `self` wholesale.
pub trait MergeFrom {
    fn merge_from(&mut self, other: &Self);

    fn merge_from_option(&mut self, other: Option<&Self>) {
        if let Some(other) = other {
            self.merge_from(other);
        }
    }
}

macro_rules! merge_from_overwrites {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl MergeFrom for $ty {
                fn merge_from(&mut self, other: &Self) {
                    *self = other.clone();
                }
            }
        )+
    };
}

merge_from_overwrites!(
    bool,
    u16,
    u32,
    u64,
    i64,
    usize,
    f32,
    f64,
    String,
    serde_json::Value,
);

impl<T: Clone + MergeFrom> MergeFrom for Option<T> {
    fn merge_from(&mut self, other: &Self) {
        let Some(other) = other else {
            return;
        };
        if let Some(this) = self {
            this.merge_from(other);
        } else {
            self.replace(other.clone());
        }
    }
}

impl<T: Clone> MergeFrom for Vec<T> {
    fn merge_from(&mut self, other: &Self) {
        *self = other.clone();
    }
}

impl<K, V> MergeFrom for BTreeMap<K, V>
where
    K: Clone + Ord,
    V: Clone + MergeFrom,
{
    fn merge_from(&mut self, other: &Self) {
        for (key, value) in other {
            if let Some(existing) = self.get_mut(key) {
                existing.merge_from(value);
            } else {
                self.insert(key.clone(), value.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_none_is_ignored() {
        let mut a = Some(1u32);
        a.merge_from(&None);
        assert_eq!(a, Some(1));
    }

    #[test]
    fn option_some_replaces() {
        let mut a = Some(1u32);
        a.merge_from(&Some(2));
        assert_eq!(a, Some(2));
    }

    #[test]
    fn option_none_becomes_some() {
        let mut a: Option<u32> = None;
        a.merge_from(&Some(2));
        assert_eq!(a, Some(2));
    }

    #[test]
    fn btreemap_merges_key_wise() {
        let mut a = BTreeMap::from([("a".to_string(), 1u32), ("b".to_string(), 2)]);
        let b = BTreeMap::from([("b".to_string(), 20u32), ("c".to_string(), 30)]);
        a.merge_from(&b);
        assert_eq!(a.get("a"), Some(&1));
        assert_eq!(a.get("b"), Some(&20));
        assert_eq!(a.get("c"), Some(&30));
    }

    #[test]
    fn vec_replaces_wholesale() {
        let mut a = vec![1, 2, 3];
        a.merge_from(&vec![9]);
        assert_eq!(a, vec![9]);
    }
}

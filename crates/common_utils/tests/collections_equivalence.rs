//! `common_utils::collections::{HashMap, HashSet}` against `std::collections`,
//! differentially.
//!
//! Each case drives the facade and std with the same input under the SAME
//! hasher value (a cloned `RandomState`, or a `BuildHasherDefault` every map
//! builds with identical keys), so both sides hold identical tables and every
//! observable must match exactly: return values, iteration order, `Debug` text,
//! serde_json bytes, capacity, equality, panics. The one intended difference,
//! the default hasher, is characterised at the end.

#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::for_kv_map,
    reason = "a differential test names std's collections on purpose, indexes freely, and walks `&mut` maps by entry to exercise that `IntoIterator`"
)]

use std::{
    collections::hash_map::RandomState,
    fmt::Debug,
    hash::{BuildHasher, BuildHasherDefault, DefaultHasher, Hash},
    panic::{catch_unwind, AssertUnwindSafe},
};

use common_utils::collections::{HashMap as FMap, HashSet as FSet};
use serde::{de::DeserializeOwned, Serialize};

type SMap<K, V, S> = std::collections::HashMap<K, V, S>;
type SSet<T, S> = std::collections::HashSet<T, S>;

/// A hasher every map builds by `Default` with the same keys, for the impls
/// that construct their own hasher (`FromIterator`, `Deserialize`, `Default`).
type Fixed = BuildHasherDefault<DefaultHasher>;

const SIZES: &[usize] = &[
    0, 1, 2, 3, 4, 7, 8, 13, 14, 15, 16, 17, 27, 28, 29, 56, 57, 100, 113, 224, 500, 1000,
];
const SEEDS: &[u64] = &[1, 0x9E37_79B9_7F4A_7C15, 0xDEAD_BEEF, 42];

/// xorshift64*: deterministic, dependency-free.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
}

/// Keys whose `Debug` and JSON renderings need escaping.
fn string_key(rng: &mut Rng, space: u64) -> String {
    let n = rng.below(space);
    match n % 4 {
        0 => format!("k{n}"),
        1 => format!("quote\"{n}"),
        2 => format!("uni-é-{n}"),
        _ => format!("tab\t{n}\n"),
    }
}

fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
        .unwrap_or_default()
}

fn assert_same_map<K, V, S>(f: &FMap<K, V, S>, s: &SMap<K, V, S>, ctx: &str)
where
    K: Debug + Serialize + Eq + Hash,
    V: Debug + Serialize + PartialEq,
    S: BuildHasher,
{
    assert_eq!(f.len(), s.len(), "{ctx}: len");
    assert_eq!(f.is_empty(), s.is_empty(), "{ctx}: is_empty");
    assert_eq!(f.capacity(), s.capacity(), "{ctx}: capacity");
    let order: Vec<(&K, &V)> = s.iter().collect();
    assert_eq!(f.iter().collect::<Vec<_>>(), order, "{ctx}: iter order");
    assert_eq!(
        IntoIterator::into_iter(f).collect::<Vec<_>>(),
        order,
        "{ctx}: IntoIterator for &Self"
    );
    assert_eq!(format!("{f:?}"), format!("{s:?}"), "{ctx}: Debug");
    assert_eq!(
        format!("{f:#?}"),
        format!("{s:#?}"),
        "{ctx}: alternate Debug"
    );
    assert_eq!(
        serde_json::to_vec(f).unwrap(),
        serde_json::to_vec(s).unwrap(),
        "{ctx}: serde_json bytes"
    );
    assert_eq!(
        serde_json::to_vec_pretty(f).unwrap(),
        serde_json::to_vec_pretty(s).unwrap(),
        "{ctx}: serde_json pretty bytes"
    );
    assert_eq!(
        serde_json::to_string(&serde_json::to_value(f).unwrap()).unwrap(),
        serde_json::to_string(&serde_json::to_value(s).unwrap()).unwrap(),
        "{ctx}: through serde_json::Value"
    );
}

fn assert_same_set<T, S>(f: &FSet<T, S>, s: &SSet<T, S>, ctx: &str)
where
    T: Debug + Serialize + Eq + Hash,
    S: BuildHasher,
{
    assert_eq!(f.len(), s.len(), "{ctx}: len");
    assert_eq!(f.is_empty(), s.is_empty(), "{ctx}: is_empty");
    assert_eq!(f.capacity(), s.capacity(), "{ctx}: capacity");
    let order: Vec<&T> = s.iter().collect();
    assert_eq!(f.iter().collect::<Vec<_>>(), order, "{ctx}: iter order");
    assert_eq!(
        IntoIterator::into_iter(f).collect::<Vec<_>>(),
        order,
        "{ctx}: IntoIterator for &Self"
    );
    assert_eq!(format!("{f:?}"), format!("{s:?}"), "{ctx}: Debug");
    assert_eq!(
        format!("{f:#?}"),
        format!("{s:#?}"),
        "{ctx}: alternate Debug"
    );
    assert_eq!(
        serde_json::to_vec(f).unwrap(),
        serde_json::to_vec(s).unwrap(),
        "{ctx}: serde_json bytes"
    );
    assert_eq!(
        serde_json::to_vec_pretty(f).unwrap(),
        serde_json::to_vec_pretty(s).unwrap(),
        "{ctx}: serde_json pretty bytes"
    );
}

// ---------------------------------------------------------------------------
// Maps: every mutation reached through DerefMut, the forwarded Extend impls,
// and the observables after each step.
// ---------------------------------------------------------------------------

fn drive_map_ops<S: BuildHasher + Clone>(hasher: &S, size: usize, seed: u64) {
    let mut rng = Rng(seed | 1);
    let mut f: FMap<u64, u32, S> = FMap::with_hasher(hasher.clone());
    let mut s: SMap<u64, u32, S> = SMap::with_hasher(hasher.clone());
    let space = (size as u64).saturating_mul(2).max(1);
    for step in 0..(size * 3 + 10) {
        let ctx = format!("u64 map, size {size}, seed {seed:#x}, step {step}");
        let k = rng.below(space);
        let v = rng.next() as u32;
        match rng.below(40) {
            0..=15 => assert_eq!(f.insert(k, v), s.insert(k, v), "{ctx}: insert"),
            16..=20 => assert_eq!(f.remove(&k), s.remove(&k), "{ctx}: remove"),
            21..=23 => assert_eq!(f.get(&k), s.get(&k), "{ctx}: get"),
            24..=26 => {
                *f.entry(k).or_insert(v) ^= 1;
                *s.entry(k).or_insert(v) ^= 1;
            }
            27 => {
                let m = rng.below(3) + 2;
                f.retain(|key, _| key % m != 0);
                s.retain(|key, _| key % m != 0);
            }
            28..=31 => {
                let batch: Vec<(u64, u32)> = (0..rng.below(20))
                    .map(|_| (rng.below(space), rng.next() as u32))
                    .collect();
                f.extend(batch.clone());
                s.extend(batch);
            }
            32..=34 => {
                let mut other: SMap<u64, u32, S> = SMap::with_hasher(hasher.clone());
                for _ in 0..rng.below(20) {
                    other.insert(rng.below(space), rng.next() as u32);
                }
                f.extend(other.iter());
                s.extend(other.iter());
            }
            35 | 36 => {
                let n = rng.below(64) as usize;
                f.reserve(n);
                s.reserve(n);
            }
            37 => {
                f.shrink_to_fit();
                s.shrink_to_fit();
            }
            38 => {
                let n = rng.below(space) as usize;
                f.shrink_to(n);
                s.shrink_to(n);
            }
            _ => {
                if rng.below(4) == 0 {
                    assert_eq!(
                        f.drain().collect::<Vec<_>>(),
                        s.drain().collect::<Vec<_>>(),
                        "{ctx}: drain order"
                    );
                }
            }
        }
        assert_same_map(&f, &s, &ctx);
    }

    let ctx = format!("u64 map, size {size}, seed {seed:#x}");
    for key in s.keys() {
        assert_eq!(f[key], s[key], "{ctx}: Index");
    }
    // A missing key panics alike. Checked on a few cases only, so the expected
    // panics don't flood stderr; a global quiet hook would also swallow real
    // assertion messages from tests running in parallel.
    if seed == SEEDS[0] && matches!(size, 0 | 17 | 1000) {
        let missing = u64::MAX;
        let fp = catch_unwind(AssertUnwindSafe(|| f[&missing])).expect_err("facade must panic");
        let sp = catch_unwind(AssertUnwindSafe(|| s[&missing])).expect_err("std must panic");
        assert_eq!(panic_text(&*fp), panic_text(&*sp), "{ctx}: Index panic");
    }

    assert_same_map(&f.clone(), &s.clone(), &format!("{ctx}: Clone"));
    let mut fc: FMap<u64, u32, S> = FMap::with_hasher(hasher.clone());
    let mut sc: SMap<u64, u32, S> = SMap::with_hasher(hasher.clone());
    for i in 0..rng.below(300) {
        fc.insert(i, 0);
        sc.insert(i, 0);
    }
    fc.clone_from(&f);
    sc.clone_from(&s);
    assert_same_map(&fc, &sc, &format!("{ctx}: clone_from"));

    for (_, v) in &mut f {
        *v = v.wrapping_add(7);
    }
    for (_, v) in &mut s {
        *v = v.wrapping_add(7);
    }
    assert_same_map(&f, &s, &format!("{ctx}: IntoIterator for &mut Self"));

    assert_eq!(
        f.clone().into_keys().collect::<Vec<_>>(),
        s.clone().into_keys().collect::<Vec<_>>(),
        "{ctx}: into_keys"
    );
    assert_eq!(
        f.clone().into_values().collect::<Vec<_>>(),
        s.clone().into_values().collect::<Vec<_>>(),
        "{ctx}: into_values"
    );
    assert_eq!(
        f.clone().into_iter().collect::<Vec<_>>(),
        s.clone().into_iter().collect::<Vec<_>>(),
        "{ctx}: IntoIterator for Self"
    );

    // Both directions of `From`, and `into_inner`, keep the table and the hasher.
    let from_std: FMap<u64, u32, S> = FMap::from(s.clone());
    assert_same_map(&from_std, &s, &format!("{ctx}: From<std>"));
    let to_std: SMap<u64, u32, S> = SMap::from(f.clone());
    assert_same_map(&f, &to_std, &format!("{ctx}: From<facade> for std"));
    let inner = f.clone().into_inner();
    assert_same_map(&f, &inner, &format!("{ctx}: into_inner"));
    assert_eq!(
        from_std.hasher().hash_one(12_345_u64),
        s.hasher().hash_one(12_345_u64),
        "{ctx}: From<std> keeps the hasher"
    );
    assert_eq!(
        inner.hasher().hash_one(12_345_u64),
        f.hasher().hash_one(12_345_u64),
        "{ctx}: into_inner keeps the hasher"
    );
}

fn drive_string_map<S: BuildHasher + Clone>(hasher: &S, size: usize, seed: u64) {
    let mut rng = Rng(seed | 1);
    let mut f: FMap<String, String, S> = FMap::with_hasher(hasher.clone());
    let mut s: SMap<String, String, S> = SMap::with_hasher(hasher.clone());
    let space = (size as u64).saturating_mul(2).max(1);
    for step in 0..(size * 2 + 5) {
        let ctx = format!("String map, size {size}, seed {seed:#x}, step {step}");
        let k = string_key(&mut rng, space);
        let v = string_key(&mut rng, 1000);
        if rng.below(5) == 0 {
            assert_eq!(f.remove(&k), s.remove(&k), "{ctx}: remove");
        } else {
            assert_eq!(
                f.insert(k.clone(), v.clone()),
                s.insert(k, v),
                "{ctx}: insert"
            );
        }
        assert_same_map(&f, &s, &ctx);
    }
}

#[test]
fn map_matches_std_under_a_shared_random_state() {
    for &size in SIZES {
        for &seed in SEEDS {
            let hasher = RandomState::new();
            drive_map_ops(&hasher, size, seed);
            drive_string_map(&hasher, size, seed);
        }
    }
}

#[test]
fn map_matches_std_under_a_fixed_hasher() {
    for &size in SIZES {
        for &seed in SEEDS {
            drive_map_ops(&Fixed::default(), size, seed);
            drive_string_map(&Fixed::default(), size, seed);
        }
    }
}

#[test]
fn map_from_iterator_matches_std() {
    for &size in SIZES {
        for &seed in SEEDS {
            let mut rng = Rng(seed | 1);
            let space = (size as u64).max(1);
            // Duplicates included: the last value for a key must win on both sides.
            let pairs: Vec<(u64, u32)> = (0..size)
                .map(|_| (rng.below(space), rng.next() as u32))
                .collect();
            let f: FMap<u64, u32, Fixed> = pairs.iter().copied().collect();
            let s: SMap<u64, u32, Fixed> = pairs.iter().copied().collect();
            assert_same_map(
                &f,
                &s,
                &format!("FromIterator, size {size}, seed {seed:#x}"),
            );
        }
    }
}

#[test]
fn map_partial_eq_matches_std() {
    for &size in SIZES {
        for &seed in SEEDS {
            let mut rng = Rng(seed | 1);
            let pairs: Vec<(u64, u32)> = (0..size).map(|i| (i as u64, rng.next() as u32)).collect();
            let hasher = RandomState::new();
            let build_f = |pairs: &[(u64, u32)]| {
                let mut m = FMap::with_hasher(hasher.clone());
                m.extend(pairs.iter().copied());
                m
            };
            let build_s = |pairs: &[(u64, u32)]| {
                let mut m = SMap::with_hasher(hasher.clone());
                m.extend(pairs.iter().copied());
                m
            };
            let reversed: Vec<(u64, u32)> = pairs.iter().rev().copied().collect();
            let mut changed = pairs.clone();
            if let Some(first) = changed.first_mut() {
                first.1 = first.1.wrapping_add(1);
            }
            let mut shorter = pairs.clone();
            shorter.pop();
            for other in [&pairs, &reversed, &changed, &shorter] {
                assert_eq!(
                    build_f(&pairs) == build_f(other),
                    build_s(&pairs) == build_s(other),
                    "PartialEq, size {size}, seed {seed:#x}"
                );
            }
        }
    }
}

fn round_trip_map<K, V>(json: &str)
where
    K: Debug + Serialize + DeserializeOwned + Eq + Hash,
    V: Debug + Serialize + DeserializeOwned + PartialEq,
{
    let f = serde_json::from_str::<FMap<K, V, Fixed>>(json);
    let s = serde_json::from_str::<SMap<K, V, Fixed>>(json);
    match (f, s) {
        (Ok(f), Ok(s)) => assert_same_map(&f, &s, &format!("Deserialize {json:?}")),
        (Err(fe), Err(se)) => {
            assert_eq!(fe.to_string(), se.to_string(), "Deserialize error {json:?}");
        }
        (f, s) => panic!("Deserialize {json:?}: facade {f:?}, std {s:?}"),
    }
}

#[test]
fn map_deserialize_matches_std() {
    for &size in SIZES {
        let mut s: SMap<u64, u32, Fixed> = SMap::default();
        let mut rng = Rng(size as u64 | 1);
        for _ in 0..size {
            s.insert(rng.below(size as u64 * 2 + 1), rng.next() as u32);
        }
        round_trip_map::<u64, u32>(&serde_json::to_string(&s).unwrap());
        let mut strings: SMap<String, String, Fixed> = SMap::default();
        for _ in 0..size {
            strings.insert(string_key(&mut rng, 500), string_key(&mut rng, 500));
        }
        round_trip_map::<String, String>(&serde_json::to_string(&strings).unwrap());
    }
    for json in [
        r#"{"2":1,"1":2,"2":3}"#,
        r#"{}"#,
        r#"[1,2]"#,
        r#"{"a":1}"#,
        r#"{"1":"x"}"#,
        r#"null"#,
        r#"{"1":1"#,
    ] {
        round_trip_map::<u64, u32>(json);
    }
}

#[test]
fn map_constructors_match_std() {
    for n in 0..2048 {
        assert_eq!(
            FMap::<u64, u32>::with_capacity(n).capacity(),
            SMap::<u64, u32, RandomState>::with_capacity(n).capacity(),
            "with_capacity({n})"
        );
        assert_eq!(
            FMap::<u64, u32, Fixed>::with_capacity_and_hasher(n, Fixed::default()).capacity(),
            SMap::<u64, u32, Fixed>::with_capacity_and_hasher(n, Fixed::default()).capacity(),
            "with_capacity_and_hasher({n})"
        );
    }
    assert_eq!(
        FMap::<u64, u32>::new().capacity(),
        SMap::<u64, u32, RandomState>::new().capacity()
    );
    assert_eq!(
        FMap::<u64, u32>::default().capacity(),
        SMap::<u64, u32, RandomState>::default().capacity()
    );
    assert_same_map(
        &FMap::<u64, u32, Fixed>::default(),
        &SMap::<u64, u32, Fixed>::default(),
        "Default",
    );

    // `From<[_; N]>` exists only for each side's default hasher, so compare
    // content; duplicates must resolve the same way.
    let f = FMap::from([(1_u64, 'a'), (2, 'b'), (1, 'c'), (3, 'd')]);
    let s = SMap::from([(1_u64, 'a'), (2, 'b'), (1, 'c'), (3, 'd')]);
    let mut fv: Vec<(u64, char)> = f.iter().map(|(k, v)| (*k, *v)).collect();
    let mut sv: Vec<(u64, char)> = s.iter().map(|(k, v)| (*k, *v)).collect();
    fv.sort_unstable();
    sv.sort_unstable();
    assert_eq!(fv, sv, "From<[_; N]>");
}

// ---------------------------------------------------------------------------
// Sets.
// ---------------------------------------------------------------------------

fn drive_set_ops<S: BuildHasher + Clone>(hasher: &S, size: usize, seed: u64) {
    let mut rng = Rng(seed | 1);
    let mut f: FSet<u64, S> = FSet::with_hasher(hasher.clone());
    let mut s: SSet<u64, S> = SSet::with_hasher(hasher.clone());
    let space = (size as u64).saturating_mul(2).max(1);
    for step in 0..(size * 3 + 10) {
        let ctx = format!("u64 set, size {size}, seed {seed:#x}, step {step}");
        let k = rng.below(space);
        match rng.below(40) {
            0..=15 => assert_eq!(f.insert(k), s.insert(k), "{ctx}: insert"),
            16..=19 => assert_eq!(f.remove(&k), s.remove(&k), "{ctx}: remove"),
            20..=22 => assert_eq!(f.contains(&k), s.contains(&k), "{ctx}: contains"),
            23 => assert_eq!(f.replace(k), s.replace(k), "{ctx}: replace"),
            24 => assert_eq!(f.take(&k), s.take(&k), "{ctx}: take"),
            25 => {
                let m = rng.below(3) + 2;
                f.retain(|x| x % m != 0);
                s.retain(|x| x % m != 0);
            }
            26..=29 => {
                let batch: Vec<u64> = (0..rng.below(20)).map(|_| rng.below(space)).collect();
                f.extend(batch.clone());
                s.extend(batch);
            }
            30..=32 => {
                let batch: Vec<u64> = (0..rng.below(20)).map(|_| rng.below(space)).collect();
                f.extend(batch.iter());
                s.extend(batch.iter());
            }
            33 | 34 => {
                let n = rng.below(64) as usize;
                f.reserve(n);
                s.reserve(n);
            }
            35 => {
                f.shrink_to_fit();
                s.shrink_to_fit();
            }
            36..=38 => {
                let mut fo: FSet<u64, S> = FSet::with_hasher(hasher.clone());
                let mut so: SSet<u64, S> = SSet::with_hasher(hasher.clone());
                for _ in 0..rng.below(30) {
                    let x = rng.below(space);
                    fo.insert(x);
                    so.insert(x);
                }
                assert_eq!(
                    f.union(&fo).collect::<Vec<_>>(),
                    s.union(&so).collect::<Vec<_>>(),
                    "{ctx}: union"
                );
                assert_eq!(
                    f.intersection(&fo).collect::<Vec<_>>(),
                    s.intersection(&so).collect::<Vec<_>>(),
                    "{ctx}: intersection"
                );
                assert_eq!(
                    f.difference(&fo).collect::<Vec<_>>(),
                    s.difference(&so).collect::<Vec<_>>(),
                    "{ctx}: difference"
                );
                assert_eq!(
                    f.symmetric_difference(&fo).collect::<Vec<_>>(),
                    s.symmetric_difference(&so).collect::<Vec<_>>(),
                    "{ctx}: symmetric_difference"
                );
                assert_eq!(f.is_subset(&fo), s.is_subset(&so), "{ctx}: is_subset");
                assert_eq!(f.is_disjoint(&fo), s.is_disjoint(&so), "{ctx}: is_disjoint");
            }
            _ => {
                if rng.below(4) == 0 {
                    assert_eq!(
                        f.drain().collect::<Vec<_>>(),
                        s.drain().collect::<Vec<_>>(),
                        "{ctx}: drain order"
                    );
                }
            }
        }
        assert_same_set(&f, &s, &ctx);
    }

    let ctx = format!("u64 set, size {size}, seed {seed:#x}");
    assert_same_set(&f.clone(), &s.clone(), &format!("{ctx}: Clone"));
    let mut fc: FSet<u64, S> = FSet::with_hasher(hasher.clone());
    let mut sc: SSet<u64, S> = SSet::with_hasher(hasher.clone());
    for i in 0..rng.below(300) {
        fc.insert(i);
        sc.insert(i);
    }
    fc.clone_from(&f);
    sc.clone_from(&s);
    assert_same_set(&fc, &sc, &format!("{ctx}: clone_from"));
    assert_eq!(
        f.clone().into_iter().collect::<Vec<_>>(),
        s.clone().into_iter().collect::<Vec<_>>(),
        "{ctx}: IntoIterator for Self"
    );
    let from_std: FSet<u64, S> = FSet::from(s.clone());
    assert_same_set(&from_std, &s, &format!("{ctx}: From<std>"));
    let to_std: SSet<u64, S> = SSet::from(f.clone());
    assert_same_set(&f, &to_std, &format!("{ctx}: From<facade> for std"));
    assert_same_set(&f, &f.clone().into_inner(), &format!("{ctx}: into_inner"));
}

#[test]
fn set_matches_std_under_a_shared_random_state() {
    for &size in SIZES {
        for &seed in SEEDS {
            drive_set_ops(&RandomState::new(), size, seed);
        }
    }
}

#[test]
fn set_matches_std_under_a_fixed_hasher() {
    for &size in SIZES {
        for &seed in SEEDS {
            drive_set_ops(&Fixed::default(), size, seed);
        }
    }
}

#[test]
fn set_from_iterator_deserialize_and_eq_match_std() {
    for &size in SIZES {
        let mut rng = Rng(size as u64 | 1);
        let values: Vec<u64> = (0..size).map(|_| rng.below(size as u64 + 1)).collect();
        let f: FSet<u64, Fixed> = values.iter().copied().collect();
        let s: SSet<u64, Fixed> = values.iter().copied().collect();
        assert_same_set(&f, &s, &format!("FromIterator, size {size}"));

        let json = serde_json::to_string(&values).unwrap();
        let fd: FSet<u64, Fixed> = serde_json::from_str(&json).unwrap();
        let sd: SSet<u64, Fixed> = serde_json::from_str(&json).unwrap();
        assert_same_set(&fd, &sd, &format!("Deserialize, size {size}"));

        let reversed: Vec<u64> = values.iter().rev().copied().collect();
        let fr: FSet<u64, Fixed> = reversed.iter().copied().collect();
        let sr: SSet<u64, Fixed> = reversed.iter().copied().collect();
        let fplus: FSet<u64, Fixed> = values.iter().copied().chain([u64::MAX]).collect();
        let splus: SSet<u64, Fixed> = values.iter().copied().chain([u64::MAX]).collect();
        assert_eq!(f == fr, s == sr, "PartialEq, reordered, size {size}");
        assert_eq!(f == fplus, s == splus, "PartialEq, grown, size {size}");
    }
    for json in ["[1,2,1]", "[]", r#"{"a":1}"#, "[\"x\"]", "null", "[1"] {
        let f = serde_json::from_str::<FSet<u64, Fixed>>(json);
        let s = serde_json::from_str::<SSet<u64, Fixed>>(json);
        match (f, s) {
            (Ok(f), Ok(s)) => assert_same_set(&f, &s, &format!("Deserialize {json:?}")),
            (Err(fe), Err(se)) => assert_eq!(fe.to_string(), se.to_string(), "{json:?}"),
            (f, s) => panic!("Deserialize {json:?}: facade {f:?}, std {s:?}"),
        }
    }
    for n in 0..2048 {
        assert_eq!(
            FSet::<u64>::with_capacity(n).capacity(),
            SSet::<u64, RandomState>::with_capacity(n).capacity(),
            "with_capacity({n})"
        );
    }
    let f = FSet::from([3_u64, 1, 2, 3]);
    let s = SSet::from([3_u64, 1, 2, 3]);
    let mut fv: Vec<u64> = f.iter().copied().collect();
    let mut sv: Vec<u64> = s.iter().copied().collect();
    fv.sort_unstable();
    sv.sort_unstable();
    assert_eq!(fv, sv, "From<[_; N]>");
}

// ---------------------------------------------------------------------------
// The trait surface, layout, and the one intended difference.
// ---------------------------------------------------------------------------

/// Whether `$ty` implements a trait, decided at compile time: an inherent
/// associated const shadows the trait's only when the bound holds.
macro_rules! implements {
    ($ty:ty: $($bound:tt)+) => {{
        struct Probe<T: ?Sized>(core::marker::PhantomData<T>);
        #[allow(dead_code)]
        trait Fallback {
            const IMPLEMENTS: bool = false;
        }
        impl<T: ?Sized> Fallback for Probe<T> {}
        #[allow(dead_code)]
        impl<T: ?Sized + $($bound)+> Probe<T> {
            const IMPLEMENTS: bool = true;
        }
        <Probe<$ty>>::IMPLEMENTS
    }};
}

macro_rules! same_surface {
    ($facade:ty, $std:ty, [$($name:literal => ($($bound:tt)+)),+ $(,)?]) => {{
        let mut rows = Vec::new();
        $(
            rows.push((
                $name,
                implements!($facade: $($bound)+),
                implements!($std: $($bound)+),
            ));
        )+
        rows
    }};
}

#[test]
fn the_trait_surface_matches_std() {
    type FM = FMap<u64, u32, RandomState>;
    type SM = SMap<u64, u32, RandomState>;
    type FS = FSet<u64, RandomState>;
    type SS = SSet<u64, RandomState>;
    let map_rows = same_surface!(FM, SM, [
        "Clone" => (Clone),
        "Debug" => (Debug),
        "Default" => (Default),
        "PartialEq" => (PartialEq),
        "Eq" => (Eq),
        "Hash" => (Hash),
        "PartialOrd" => (PartialOrd),
        "Ord" => (Ord),
        "Copy" => (Copy),
        "Display" => (std::fmt::Display),
        "Send" => (Send),
        "Sync" => (Sync),
        "Unpin" => (Unpin),
        "UnwindSafe" => (std::panic::UnwindSafe),
        "RefUnwindSafe" => (std::panic::RefUnwindSafe),
        "Serialize" => (Serialize),
        "DeserializeOwned" => (DeserializeOwned),
        "FromIterator<(K, V)>" => (FromIterator<(u64, u32)>),
        "Extend<(K, V)>" => (Extend<(u64, u32)>),
        "Extend<(&K, &V)>" => (Extend<(&'static u64, &'static u32)>),
        "IntoIterator" => (IntoIterator<Item = (u64, u32)>),
        "Index<&K>" => (std::ops::Index<&'static u64, Output = u32>),
        "utoipa::ToSchema" => (for<'s> utoipa::ToSchema<'s>),
    ]);
    let set_rows = same_surface!(FS, SS, [
        "Clone" => (Clone),
        "Debug" => (Debug),
        "Default" => (Default),
        "PartialEq" => (PartialEq),
        "Eq" => (Eq),
        "Hash" => (Hash),
        "PartialOrd" => (PartialOrd),
        "Ord" => (Ord),
        "Send" => (Send),
        "Sync" => (Sync),
        "Unpin" => (Unpin),
        "UnwindSafe" => (std::panic::UnwindSafe),
        "RefUnwindSafe" => (std::panic::RefUnwindSafe),
        "Serialize" => (Serialize),
        "DeserializeOwned" => (DeserializeOwned),
        "FromIterator<T>" => (FromIterator<u64>),
        "Extend<T>" => (Extend<u64>),
        "Extend<&T>" => (Extend<&'static u64>),
        "IntoIterator" => (IntoIterator<Item = u64>),
    ]);
    // `&a | &b` and friends: std defines them for `&HashSet`, the facade does not.
    // Code using them on the facade fails to compile, so no behaviour can change
    // silently; recorded here so the table is complete.
    let operator_rows = [
        (
            "&Set: BitOr",
            implements!(&'static FS: std::ops::BitOr<&'static FS>),
            implements!(&'static SS: std::ops::BitOr<&'static SS>),
        ),
        (
            "&Set: BitAnd",
            implements!(&'static FS: std::ops::BitAnd<&'static FS>),
            implements!(&'static SS: std::ops::BitAnd<&'static SS>),
        ),
        (
            "&Set: BitXor",
            implements!(&'static FS: std::ops::BitXor<&'static FS>),
            implements!(&'static SS: std::ops::BitXor<&'static SS>),
        ),
        (
            "&Set: Sub",
            implements!(&'static FS: std::ops::Sub<&'static FS>),
            implements!(&'static SS: std::ops::Sub<&'static SS>),
        ),
    ];
    let mut differences = Vec::new();
    for (name, facade, std_side) in map_rows
        .iter()
        .map(|r| ("map", r))
        .chain(set_rows.iter().map(|r| ("set", r)))
        .map(|(kind, (n, f, s))| (format!("{kind} {n}"), *f, *s))
        .chain(
            operator_rows
                .iter()
                .map(|(n, f, s)| ((*n).to_owned(), *f, *s)),
        )
    {
        println!("{name:<32} facade={facade:<5} std={std_side}");
        if facade != std_side {
            differences.push(name);
        }
    }
    assert_eq!(
        differences,
        vec!["&Set: BitOr", "&Set: BitAnd", "&Set: BitXor", "&Set: Sub"],
        "the facade's trait surface differs from std's somewhere other than the set operators"
    );
}

#[test]
fn layout_matches_std() {
    use std::mem::{align_of, size_of};
    assert_eq!(
        size_of::<FMap<u64, u32, RandomState>>(),
        size_of::<SMap<u64, u32, RandomState>>()
    );
    assert_eq!(
        align_of::<FMap<u64, u32, RandomState>>(),
        align_of::<SMap<u64, u32, RandomState>>()
    );
    assert_eq!(
        size_of::<FSet<u64, RandomState>>(),
        size_of::<SSet<u64, RandomState>>()
    );
    assert_eq!(
        size_of::<common_utils::collections::DefaultHashBuilder>(),
        size_of::<RandomState>(),
        "the default hasher is the same size as the one it replaces"
    );
}

/// Outside a recorded or replayed correlation every map draws fresh keys, as
/// `RandomState::new()` does: per map, not per process.
fn fresh_keys_per_map(label: &str) {
    const N: usize = 2000;
    let probes: Vec<u64> = (0..N)
        .map(|i| match i % 6 {
            0 => FMap::<u64, ()>::new().hasher().hash_one(7_u64),
            1 => FMap::<u64, ()>::default().hasher().hash_one(7_u64),
            2 => FMap::<u64, ()>::with_capacity(8).hasher().hash_one(7_u64),
            3 => FSet::<u64>::new().hasher().hash_one(7_u64),
            4 => [(1_u64, ())]
                .into_iter()
                .collect::<FMap<u64, ()>>()
                .hasher()
                .hash_one(7_u64),
            _ => serde_json::from_str::<FSet<u64>>("[1]")
                .unwrap()
                .hasher()
                .hash_one(7_u64),
        })
        .collect();
    let distinct: std::collections::BTreeSet<u64> = probes.iter().copied().collect();
    assert_eq!(distinct.len(), N, "{label}: some maps shared keys");

    let other_thread = std::thread::spawn(|| FMap::<u64, ()>::new().hasher().hash_one(7_u64))
        .join()
        .unwrap();
    assert!(
        !distinct.contains(&other_thread),
        "{label}: another thread reused keys"
    );
}

#[test]
fn outside_a_correlation_every_map_draws_fresh_keys() {
    fresh_keys_per_map("no correlation");
}

/// With deja compiled in but idle (no hook installed), which is how a build
/// with the feature runs outside record and replay, a correlation in scope does
/// not change that: the guard reads the runtime mode first.
#[cfg(feature = "deja")]
#[test]
fn a_correlation_with_deja_idle_still_draws_fresh_keys() {
    assert!(
        deja::runtime_mode_is_disabled(),
        "this binary installs no hook"
    );
    let _correlation = deja::test_support::recording_correlation("corr-equivalence-audit");
    assert_eq!(
        deja::try_current_correlation_id().as_deref(),
        Some("corr-equivalence-audit")
    );
    fresh_keys_per_map("correlation in scope, deja idle");
}

/// The hasher's `Debug`: std hides `RandomState`'s keys, the derived `Debug` on
/// `CorrelationHasher` prints them. Recorded rather than asserted equal.
#[cfg(feature = "deja")]
#[test]
fn the_hashers_debug_differs_and_only_there() {
    let std_text = format!("{:?}", RandomState::new());
    let ours = format!(
        "{:?}",
        common_utils::collections::DefaultHashBuilder::default()
    );
    println!("std: {std_text}\nfacade: {ours}");
    assert_eq!(std_text, "RandomState { .. }");
    assert!(ours.starts_with("CorrelationHasher { k0: "), "{ours}");
}

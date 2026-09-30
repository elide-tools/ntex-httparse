//! Escalate long fields without feature-selection overhead on the common short-field path.
use crate::{State, iter::Bytes};
use std::sync::atomic::{AtomicU8, Ordering};

static FEATURE: AtomicU8 = AtomicU8::new(0);
const NONE: u8 = 1;
const BW: u8 = 2;
const VBMI: u8 = 3;

#[inline]
fn feature() -> u8 {
    let found = FEATURE.load(Ordering::Relaxed);
    if found != 0 {
        return found;
    }
    let found = if is_x86_feature_detected!("avx512f") && is_x86_feature_detected!("avx512bw") {
        if is_x86_feature_detected!("avx512vbmi") {
            VBMI
        } else {
            BW
        }
    } else {
        NONE
    };
    FEATURE.store(found, Ordering::Relaxed);
    found
}

#[inline]
pub(super) fn match_uri(input: &[u8]) -> Option<usize> {
    if input.len() < 16 || feature() == NONE {
        return None;
    }
    // SAFETY: CPU/OS feature detection guards the target-feature function.
    let mut state = State::default();
    let mut bytes = Bytes::new(input, &mut state);
    unsafe {
        super::avx512::match_uri_vectored(&mut bytes);
    }
    Some(bytes.cursor())
}

#[inline]
pub(super) fn match_header_value(input: &[u8]) -> Option<usize> {
    if input.len() < 16 || feature() == NONE {
        return None;
    }
    // SAFETY: CPU/OS feature detection guards the target-feature function.
    let mut state = State::default();
    let mut bytes = Bytes::new(input, &mut state);
    unsafe {
        super::avx512::match_header_value_vectored(&mut bytes);
    }
    Some(bytes.cursor())
}

#[inline]
pub(super) fn match_header_name(input: &[u8]) -> Option<usize> {
    if input.len() < 16 || feature() != VBMI {
        return None;
    }
    // SAFETY: this scanner additionally requires the detected VBMI byte permutations.
    let mut state = State::default();
    let mut bytes = Bytes::new(input, &mut state);
    unsafe {
        super::avx512::match_header_name_vectored(&mut bytes);
    }
    Some(bytes.cursor())
}

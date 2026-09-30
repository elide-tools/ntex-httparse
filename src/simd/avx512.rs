use crate::iter::Bytes;
#[cfg(target_arch = "x86")]
use core::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

#[inline]
#[target_feature(enable = "avx512f,avx512bw")]
pub unsafe fn match_uri_vectored(bytes: &mut Bytes) {
    scan::<false>(bytes);
}

#[inline]
#[target_feature(enable = "avx512f,avx512bw")]
pub unsafe fn match_header_value_vectored(bytes: &mut Bytes) {
    scan::<true>(bytes);
}

#[inline]
#[target_feature(enable = "avx512f,avx512bw")]
unsafe fn scan<const HEADER: bool>(bytes: &mut Bytes) {
    let input = bytes.as_ref();
    let mut offset = 0;
    while input.len() - offset >= 16 {
        let length = (input.len() - offset).min(64);
        let active = u64::MAX >> (64 - length);
        // Masked-off lanes do not access memory, including across an unmapped page.
        let data = _mm512_maskz_loadu_epi8(active, input.as_ptr().add(offset).cast());
        let low = _mm512_cmpge_epu8_mask(data, _mm512_set1_epi8(if HEADER { 32 } else { 33 }));
        let tab = if HEADER {
            _mm512_cmpeq_epi8_mask(data, _mm512_set1_epi8(9))
        } else {
            0
        };
        let del = _mm512_cmpeq_epi8_mask(data, _mm512_set1_epi8(127));
        let valid = (low | tab) & !del & active;
        if valid != active {
            bytes.advance(offset + valid.trailing_ones() as usize);
            return;
        }
        // Keep the next load independent of the mask-to-integer bit scan on full blocks.
        offset += length;
    }
    bytes.advance(offset);
    if HEADER {
        super::swar::match_header_value_vectored(bytes);
    } else {
        super::swar::match_uri_vectored(bytes);
    }
}

const fn name_table() -> [u8; 128] {
    let mut table = [0; 128];
    let mut i = 0;
    while i < table.len() {
        table[i] = if matches!(i as u8, b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' |
            b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' | b'.' |
            b'^' | b'_' | b'`' | b'|' | b'~')
        {
            255
        } else {
            0
        };
        i += 1;
    }
    table
}
const NAMES: [u8; 128] = name_table();

#[inline]
#[target_feature(enable = "avx512f,avx512bw,avx512vbmi")]
pub unsafe fn match_header_name_vectored(bytes: &mut Bytes) {
    let low = _mm512_loadu_si512(NAMES.as_ptr().cast());
    let high = _mm512_loadu_si512(NAMES.as_ptr().add(64).cast());
    let input = bytes.as_ref();
    let mut offset = 0;
    while input.len() - offset >= 16 {
        let length = (input.len() - offset).min(64);
        let active = u64::MAX >> (64 - length);
        let data = _mm512_maskz_loadu_epi8(active, input.as_ptr().add(offset).cast());
        let classified = _mm512_permutex2var_epi8(low, data, high);
        let valid = _mm512_movepi8_mask(classified) & !_mm512_movepi8_mask(data) & active;
        if valid != active {
            bytes.advance(offset + valid.trailing_ones() as usize);
            return;
        }
        offset += length;
    }
    bytes.advance(offset);
    super::swar::match_header_name_vectored(bytes);
}

use super::*;
use crate::{State, iter::Bytes};
use std::hint::black_box;
use std::time::Instant;

type Scan = for<'a, 'b> unsafe fn(&mut Bytes<'a, 'b>);

fn backends() -> Vec<(&'static str, [Scan; 3])> {
    #[allow(unused_mut)]
    let mut result: Vec<(&str, [Scan; 3])> = vec![
        (
            "swar",
            [
                swar::match_uri_vectored,
                swar::match_header_value_vectored,
                swar::match_header_name_vectored,
            ],
        ),
        (
            "dispatch",
            [
                match_uri_vectored,
                match_header_value_vectored,
                match_header_name_vectored,
            ],
        ),
    ];
    #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
    {
        #[cfg(not(target_feature = "avx2"))]
        if is_x86_feature_detected!("sse4.2") {
            result.push((
                "sse42",
                [
                    sse42::match_uri_vectored,
                    sse42::match_header_value_vectored,
                    swar::match_header_name_vectored,
                ],
            ));
        }
        if is_x86_feature_detected!("avx2") {
            result.push((
                "avx2",
                [
                    avx2::match_uri_vectored,
                    avx2::match_header_value_vectored,
                    swar::match_header_name_vectored,
                ],
            ));
        }
        if is_x86_feature_detected!("avx512f") && is_x86_feature_detected!("avx512bw") {
            result.push((
                "avx512bw",
                [
                    avx512::match_uri_vectored,
                    avx512::match_header_value_vectored,
                    swar::match_header_name_vectored,
                ],
            ));
            if is_x86_feature_detected!("avx512vbmi") {
                result.push((
                    "avx512vbmi",
                    [
                        avx512::match_uri_vectored,
                        avx512::match_header_value_vectored,
                        avx512::match_header_name_vectored,
                    ],
                ));
            }
        }
    }
    #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
    result.push((
        "neon",
        [
            neon::match_uri_vectored,
            neon::match_header_value_vectored,
            neon::match_header_name_vectored,
        ],
    ));
    result
}

fn scanned(scan: Scan, input: &[u8]) -> usize {
    let mut state = State::default();
    let mut bytes = Bytes::new(input, &mut state);
    // SAFETY: callers select only a backend whose required features were detected.
    unsafe {
        scan(&mut bytes);
    }
    bytes.cursor()
}

#[test]
fn every_backend_matches_byte_tables_at_vector_and_slice_boundaries() {
    let tables = [
        &crate::utils::URI_MAP,
        &crate::utils::HEADER_VALUE_MAP,
        &crate::utils::TOKEN_MAP,
    ];
    for (backend, scans) in backends() {
        for (kind, scan) in scans.into_iter().enumerate() {
            for length in [
                0, 1, 7, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129,
            ] {
                for offset in [0, 1, 7, 15, 31, 63] {
                    let mut allocation = vec![b'a'; offset + length];
                    let input = &mut allocation[offset..];
                    assert_eq!(scanned(scan, input), length, "{backend} {kind} {length}");
                    for index in [0, length / 2, length.saturating_sub(1)] {
                        if index >= length {
                            continue;
                        }
                        for byte in 0..=255u8 {
                            input[index] = byte;
                            let expected = if tables[kind][usize::from(byte)] {
                                length
                            } else {
                                index
                            };
                            assert_eq!(
                                scanned(scan, input),
                                expected,
                                "{backend} kind={kind} length={length} index={index} byte={byte}"
                            );
                        }
                        input[index] = b'a';
                    }
                }
            }
        }
    }
}

#[test]
#[ignore = "explicit native microbenchmark"]
fn benchmark_backends() {
    let count = 300_000;
    for (backend, scans) in backends() {
        for (kind, scan) in scans.into_iter().enumerate() {
            for (field, total) in [
                (4, 128),
                (12, 128),
                (24, 128),
                (48, 128),
                (96, 256),
                (1024, 2048),
                (15, 15),
                (31, 31),
                (63, 63),
            ] {
                let mut data = vec![b'a'; total];
                if field < total {
                    data[field] = if kind == 0 {
                        b' '
                    } else if kind == 1 {
                        b'\r'
                    } else {
                        b':'
                    };
                }
                let start = Instant::now();
                for _ in 0..count {
                    black_box(scanned(black_box(scan), black_box(&data)));
                }
                println!(
                    "SCAN,{backend},{kind},{field},{total},{:.3}",
                    start.elapsed().as_nanos() as f64 / f64::from(count)
                );
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn scanner_tails_do_not_read_the_next_page() {
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
    let mapping = unsafe {
        libc::mmap(
            core::ptr::null_mut(),
            page * 2,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANON,
            -1,
            0,
        )
    };
    assert_ne!(mapping, libc::MAP_FAILED);
    let base = mapping.cast::<u8>();
    assert_eq!(
        unsafe { libc::mprotect(base.add(page).cast(), page, libc::PROT_NONE) },
        0
    );
    unsafe {
        base.write_bytes(b'a', page);
    }
    for (_, scans) in backends() {
        for scan in scans {
            for length in 0..=129 {
                let input = unsafe { core::slice::from_raw_parts(base.add(page - length), length) };
                assert_eq!(scanned(scan, input), length);
            }
        }
    }
    assert_eq!(unsafe { libc::munmap(mapping, page * 2) }, 0);
}

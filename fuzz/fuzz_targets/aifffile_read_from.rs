#![no_main]

use std::io::Cursor;

use libfuzzer_sys::fuzz_target;
use lofty::{AudioFile, ParseOptions};

fuzz_target!(|data: Vec<u8>| {
	let _ = lofty::iff::aiff::AiffFile::read_from(&mut Cursor::new(data), ParseOptions::new());
});

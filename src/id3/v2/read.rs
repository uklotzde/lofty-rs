use super::frame::read::ParsedFrame;
use super::header::Id3v2Header;
use super::tag::Id3v2Tag;
use crate::error::{Id3v2Error, Id3v2ErrorKind, Result};
use crate::id3::v2::util::synchsafe::UnsynchronizedStream;
use crate::probe::ParsingMode;

use std::io::Read;

pub(crate) fn parse_id3v2<R>(
	bytes: &mut R,
	header: Id3v2Header,
	parse_mode: ParsingMode,
) -> Result<Id3v2Tag>
where
	R: Read,
{
	log::debug!(
		"Parsing ID3v2 tag, size: {}, version: {:?}",
		header.size,
		header.version
	);

	let mut tag_bytes = bytes.take(u64::from(header.size - header.extended_size));

	let ret;
	if header.flags.unsynchronisation {
		// Unsynchronize the entire tag
		let mut unsynchronized_reader = UnsynchronizedStream::new(tag_bytes);
		ret = read_all_frames_into_tag(&mut unsynchronized_reader, header, parse_mode)?;

		// Get the `Take` back from the `UnsynchronizedStream`
		tag_bytes = unsynchronized_reader.into_inner();
	} else {
		ret = read_all_frames_into_tag(&mut tag_bytes, header, parse_mode)?;
	};

	// Throw away the rest of the tag (padding, bad frames)
	std::io::copy(&mut tag_bytes, &mut std::io::sink())?;
	Ok(ret)
}

fn skip_frame(reader: &mut impl Read, size: u32) -> Result<()> {
	log::trace!("Skipping frame of size {}", size);

	let size = u64::from(size);
	let mut reader = reader.take(size);
	let skipped = std::io::copy(&mut reader, &mut std::io::sink())?;
	debug_assert!(skipped <= size);
	if skipped != size {
		return Err(Id3v2Error::new(Id3v2ErrorKind::BadFrameLength).into());
	}
	Ok(())
}

fn read_all_frames_into_tag<R>(
	reader: &mut R,
	header: Id3v2Header,
	parse_mode: ParsingMode,
) -> Result<Id3v2Tag>
where
	R: Read,
{
	let mut tag = Id3v2Tag::default();
	tag.original_version = header.version;
	tag.set_flags(header.flags);

	loop {
		match ParsedFrame::read(reader, header.version, parse_mode)? {
			ParsedFrame::Next(frame) => drop(tag.insert(frame)),
			// No frame content found or ignored due to errors, but we can expect more frames
			ParsedFrame::Skip { size } => {
				skip_frame(reader, size)?;
			},
			// No frame content found, and we can expect there are no more frames
			ParsedFrame::Eof => break,
		}
	}

	Ok(tag)
}

#[test]
fn zero_size_id3v2() {
	use crate::id3::v2::header::Id3v2Header;
	use crate::ParsingMode;
	use std::io::Cursor;

	let mut f = Cursor::new(std::fs::read("tests/tags/assets/id3v2/zero.id3v2").unwrap());
	let header = Id3v2Header::parse(&mut f).unwrap();
	assert!(parse_id3v2(&mut f, header, ParsingMode::Strict).is_ok());
}

#[test]
fn bad_frame_id_relaxed_id3v2() {
	use crate::id3::v2::header::Id3v2Header;
	use crate::{Accessor, ParsingMode, TagExt};
	use std::io::Cursor;

	// Contains a frame with a "+" in the ID, which is invalid.
	// All other frames in the tag are valid, however.
	let mut f = Cursor::new(
		std::fs::read("tests/tags/assets/id3v2/bad_frame_otherwise_valid.id3v24").unwrap(),
	);
	let header = Id3v2Header::parse(&mut f).unwrap();
	let id3v2 = parse_id3v2(&mut f, header, ParsingMode::Relaxed);
	assert!(id3v2.is_ok());

	let id3v2 = id3v2.unwrap();

	// There are 6 valid frames and 1 invalid frame
	assert_eq!(id3v2.len(), 6);

	assert_eq!(id3v2.title().as_deref(), Some("Foo title"));
	assert_eq!(id3v2.artist().as_deref(), Some("Bar artist"));
	assert_eq!(id3v2.comment().as_deref(), Some("Qux comment"));
	assert_eq!(id3v2.year(), Some(1984));
	assert_eq!(id3v2.track(), Some(1));
	assert_eq!(id3v2.genre().as_deref(), Some("Classical"));
}

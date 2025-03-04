use crate::error::Result;
use crate::id3::v2::frame::content::verify_encoding;
use crate::id3::v2::header::Id3v2Version;
use crate::util::text::{decode_text, encode_text, TextDecodeOptions, TextEncoding};

use std::hash::{Hash, Hasher};
use std::io::Read;

use byteorder::ReadBytesExt;

/// An extended `ID3v2` URL frame
///
/// This is used in the `WXXX` frame, where the frames
/// are told apart by descriptions, rather than their [`FrameId`](crate::id3::v2::FrameId)s.
/// This means for each `ExtendedUrlFrame` in the tag, the description
/// must be unique.
#[derive(Clone, Debug, Eq)]
pub struct ExtendedUrlFrame {
	/// The encoding of the description and comment text
	pub encoding: TextEncoding,
	/// Unique content description
	pub description: String,
	/// The actual frame content
	pub content: String,
}

impl PartialEq for ExtendedUrlFrame {
	fn eq(&self, other: &Self) -> bool {
		self.description == other.description
	}
}

impl Hash for ExtendedUrlFrame {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.description.hash(state);
	}
}

impl ExtendedUrlFrame {
	/// Read an [`ExtendedUrlFrame`] from a slice
	///
	/// NOTE: This expects the frame header to have already been skipped
	///
	/// # Errors
	///
	/// * Unable to decode the text
	///
	/// ID3v2.2:
	///
	/// * The encoding is not [`TextEncoding::Latin1`] or [`TextEncoding::UTF16`]
	pub fn parse<R>(reader: &mut R, version: Id3v2Version) -> Result<Option<Self>>
	where
		R: Read,
	{
		let Ok(encoding_byte) = reader.read_u8() else {
			return Ok(None);
		};

		let encoding = verify_encoding(encoding_byte, version)?;
		let description = decode_text(
			reader,
			TextDecodeOptions::new().encoding(encoding).terminated(true),
		)?
		.content;
		let content = decode_text(
			reader,
			TextDecodeOptions::new().encoding(TextEncoding::Latin1),
		)?
		.content;

		Ok(Some(ExtendedUrlFrame {
			encoding,
			description,
			content,
		}))
	}

	/// Convert an [`ExtendedUrlFrame`] to a byte vec
	pub fn as_bytes(&self) -> Vec<u8> {
		let mut bytes = vec![self.encoding as u8];

		bytes.extend(encode_text(&self.description, self.encoding, true).iter());
		bytes.extend(encode_text(&self.content, self.encoding, false));

		bytes
	}
}

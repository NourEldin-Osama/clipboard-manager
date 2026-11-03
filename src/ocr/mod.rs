//! Reading text in images, with the reader chosen in Settings: Windows' own,
//! or a vision model on a server.

use anyhow::Result;

mod server;
#[cfg(windows)]
mod windows_reader;

pub use server::{test_read, ServerReader, TEST_IMAGE};
#[cfg(windows)]
pub use windows_reader::WindowsReader;

/// A text reader, ready to use.
pub enum Reader {
    #[cfg(windows)]
    Windows(WindowsReader),
    Server(ServerReader),
}

impl Reader {
    pub fn recognize(&self, img: &image::DynamicImage) -> Result<String> {
        match self {
            #[cfg(windows)]
            Self::Windows(reader) => reader.recognize(img),
            Self::Server(reader) => reader.recognize(img),
        }
    }
}

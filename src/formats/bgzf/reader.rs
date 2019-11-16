use std::io::{self, Read, Seek};

use byteorder::{ByteOrder, LittleEndian};
use flate2::read::DeflateDecoder;

use super::Block;

pub struct Reader<R: Read + Seek> {
    inner: R,
    position: u64,
    buf: Vec<u8>,
}

impl<R: Read + Seek> Reader<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            position: 0,
            buf: Vec::new(),
        }
    }

    pub fn position(&self) -> u64 {
        self.position
    }

    pub fn read_block(&mut self, block: &mut Block) -> io::Result<usize> {
        // gzip header
        let mut header = [0; 18];
        if let Err(_) = self.inner.read_exact(&mut header) {
            return Ok(0);
        }

        let bsize = &header[16..18];
        let block_size = LittleEndian::read_u16(bsize);

        let buf_size = block_size as usize - 6 - 18 - 1;

        self.buf.resize(buf_size, Default::default());
        self.inner.read_exact(&mut self.buf)?;

        let mut trailer = [0; 8];
        self.inner.read_exact(&mut trailer)?;

        let mut decoder = DeflateDecoder::new(&self.buf[..]);

        let block_buf = block.get_mut();
        block_buf.clear();

        decoder.read_to_end(block_buf)?;

        block.set_position(0);

        self.position += header.len() as u64 + self.buf.len() as u64 + trailer.len() as u64;

        Ok(block_size as usize)
    }
}

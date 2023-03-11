use byteorder::{LittleEndian, ReadBytesExt};
use positioned_io::{Cursor, ReadAt, Slice};
use std::fs::OpenOptions;

struct Reader<IO> {
    inner: IO,
}

impl<IO: ReadAt> Reader<IO> {
    fn new(inner: IO) -> Self {
        Self { inner }
    }

    fn u16(&self, offset: u64) -> color_eyre::Result<u16> {
        let mut cursor = Cursor::new_pos(&self.inner, offset);
        Ok(cursor.read_u16::<LittleEndian>()?)
    }

    fn u32(&self, offset: u64) -> color_eyre::Result<u32> {
        let mut cursor = Cursor::new_pos(&self.inner, offset);
        Ok(cursor.read_u32::<LittleEndian>()?)
    }
}

fn main() -> color_eyre::Result<()> {
    let file = OpenOptions::new().read(true).open("/dev/sda3")?;

    let r = Reader::new(Slice::new(file, 1024, None));

    let magic = r.u16(0x38)?;
    println!("magic = {magic:x}");

    let n = r.u32(0x18)?;
    let block_size = 1 << (10 + n);
    println!("block_size = {block_size}");

    let bpg = r.u32(0x20)?;
    println!("blocks per group = {bpg}");

    let ipg = r.u32(0x28)?;
    println!("inodes per group = {ipg}");

    Ok(())
}

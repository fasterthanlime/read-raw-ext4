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
}

fn main() -> color_eyre::Result<()> {
    let file = OpenOptions::new().read(true).open("/dev/sda3")?;

    // create a slice that corresponds to the superblock
    let r = Reader::new(Slice::new(file, 1024, None));

    // as per the docs
    let magic = r.u16(0x38)?;
    println!("magic = {:x}", magic);

    Ok(())
}

use byteorder::{LittleEndian, ReadBytesExt};
use custom_debug::Debug as CustomDebug;
use positioned_io::{Cursor, ReadAt, Slice};
use std::fs::OpenOptions;

#[derive(CustomDebug)]
struct Superblock {
    #[debug(format = "{:x}")]
    magic: u16,
    block_size: u64,
    blocks_per_group: u64,
    inodes_per_group: u64,
    inode_size: u64,
}

impl Superblock {
    fn new(dev: &dyn ReadAt) -> color_eyre::Result<Self> {
        let r = Reader::new(Slice::new(dev, 1024, None));
        // note: we're casting a few fields to `u64` now.
        // this will save us a bunch of grief later.
        Ok(Self {
            magic: r.u16(0x38)?,
            block_size: (2u32.pow(10 + r.u32(0x18)?)) as u64,
            blocks_per_group: r.u32(0x20)? as u64,
            inodes_per_group: r.u32(0x28)? as u64,
            inode_size: r.u16(0x58)? as u64,
        })
    }
}

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
    // open our ext4 partition, READ-ONLY.
    let file = OpenOptions::new().read(true).open("/dev/sda3")?;

    let sb = Superblock::new(&file)?;
    println!("{sb:#?}");

    Ok(())
}

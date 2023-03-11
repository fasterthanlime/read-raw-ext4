use byteorder::{LittleEndian, ReadBytesExt};
use color_eyre::Result;
use custom_debug::Debug as CustomDebug;
use num_enum::*;
use positioned_io::{Cursor, ReadAt, Slice};
use std::convert::TryFrom;
use std::fs::OpenOptions;

#[derive(CustomDebug)]
struct DirectoryEntry {
    #[debug(skip)]
    len: u64,
    inode: InodeNumber,
    name: String,
}

impl DirectoryEntry {
    fn new(slice: &dyn ReadAt) -> Result<Self> {
        let r = Reader::new(slice);
        let name_len = r.u8(0x6)? as usize;
        Ok(Self {
            inode: InodeNumber(r.u32(0x0)? as u64),
            len: r.u16(0x4)? as u64,
            name: String::from_utf8_lossy(&r.vec(0x8, name_len)?).into(),
        })
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct Extent {
    len: u64,
    start: u64,
}

impl Extent {
    fn new(slice: &dyn ReadAt) -> Result<Self> {
        let r = Reader::new(slice);
        Ok(Self {
            len: r.u16(0x4)? as u64,
            // the block number the extent points to is split
            // between upper 16-bits and lower 32-bits.
            start: ((r.u16(0x6)? as u64) << 32) + r.u32(0x8)? as u64,
        })
    }
}

#[derive(Debug)]
struct ExtentHeader {
    entries: u64,
    depth: u64,
}

impl ExtentHeader {
    fn new(slice: &dyn ReadAt) -> Result<Self> {
        let r = Reader::new(slice);
        let magic = r.u16(0x0)?;
        assert_eq!(magic, 0xF30A);

        Ok(Self {
            entries: r.u16(0x2)? as u64,
            depth: r.u16(0x6)? as u64,
        })
    }
}

#[derive(Debug, TryFromPrimitive)]
#[repr(u16)]
enum Filetype {
    Fifo = 0x1000,
    CharacterDevice = 0x2000,
    Directory = 0x4000,
    BlockDevice = 0x6000,
    Regular = 0x8000,
    SymbolicLink = 0xA000,
    Socket = 0xC000,
}

#[derive(CustomDebug)]
struct Inode {
    #[debug(format = "{:o}")]
    mode: u16,
    size: u64,

    #[debug(skip)]
    #[allow(dead_code)]
    block: Vec<u8>,
}

impl Inode {
    fn new(slice: &dyn ReadAt) -> Result<Self> {
        let r = Reader::new(slice);
        Ok(Self {
            mode: r.u16(0x0)?,
            size: r.u64_lohi(0x4, 0x6C)?,
            block: r.vec(0x28, 60)?,
        })
    }

    fn filetype(&self) -> Filetype {
        Filetype::try_from(self.mode & 0xF000).unwrap()
    }

    fn data<T>(&self, sb: &Superblock, dev: T) -> Result<Slice<T>>
    where
        T: ReadAt,
    {
        let ext_header = ExtentHeader::new(&Slice::new(&self.block, 0, Some(12)))?;
        assert_eq!(ext_header.depth, 0);
        assert_eq!(ext_header.entries, 1);

        let ext = Extent::new(&Slice::new(&self.block, 12, Some(12)))?;
        assert_eq!(ext.len, 1);

        let offset = ext.start * sb.block_size;
        let len = ext.len * sb.block_size;
        Ok(Slice::new(dev, offset, Some(len)))
    }

    fn dir_entries(&self, sb: &Superblock, dev: &dyn ReadAt) -> Result<Vec<DirectoryEntry>> {
        let data = self.data(sb, dev)?;

        let mut entries = Vec::new();
        let mut offset: u64 = 0;
        loop {
            let entry = DirectoryEntry::new(&Slice::new(&data, offset, None))?;
            if entry.inode.0 == 0 {
                break;
            }
            offset += entry.len;
            entries.push(entry);
        }
        Ok(entries)
    }
}

#[derive(Debug, Clone, Copy)]
struct InodeNumber(u64);

impl InodeNumber {
    fn blockgroup_number(self, sb: &Superblock) -> BlockGroupNumber {
        let n = (self.0 - 1) / sb.inodes_per_group;
        BlockGroupNumber(n)
    }

    // in impl InodeNumber
    fn inode_slice<T>(self, sb: &Superblock, dev: T) -> Result<Slice<T>>
    where
        T: ReadAt,
    {
        let desc = self.blockgroup_number(sb).desc(sb, &dev)?;
        let table_off = desc.inode_table * sb.block_size;
        let idx_in_table = (self.0 - 1) % sb.inodes_per_group;
        let inode_off = table_off + sb.inode_size * idx_in_table;
        Ok(Slice::new(dev, inode_off, Some(sb.inode_size)))
    }

    // in impl InodeNumber
    fn inode(self, sb: &Superblock, dev: &dyn ReadAt) -> Result<Inode> {
        let slice = self.inode_slice(sb, dev)?;
        Inode::new(&slice)
    }
}

#[derive(Debug, Clone, Copy)]
struct BlockGroupNumber(u64);

impl BlockGroupNumber {
    fn desc_slice<T>(self, sb: &Superblock, dev: T) -> Slice<T>
    where
        T: ReadAt,
    {
        assert!(sb.block_size != 1024, "1024 block size not supported");
        // the superblock takes up 1 block
        let gdt_start = sb.block_size;
        let offset = gdt_start + self.0 * BlockGroupDescriptor::SIZE;
        Slice::new(dev, offset, None)
    }

    fn desc(self, sb: &Superblock, dev: &dyn ReadAt) -> Result<BlockGroupDescriptor> {
        let slice = self.desc_slice(sb, dev);
        BlockGroupDescriptor::new(&slice)
    }
}

#[derive(Debug)]
struct BlockGroupDescriptor {
    #[allow(dead_code)]
    inode_table: u64,
}

impl BlockGroupDescriptor {
    const SIZE: u64 = 64;

    fn new(slice: &dyn ReadAt) -> Result<Self> {
        let r = Reader::new(slice);
        Ok(Self {
            inode_table: r.u64_lohi(0x8, 0x28)?,
        })
    }
}

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
    fn new(dev: &dyn ReadAt) -> Result<Self> {
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

    fn u8(&self, offset: u64) -> Result<u8> {
        let mut cursor = Cursor::new_pos(&self.inner, offset);
        Ok(cursor.read_u8()?)
    }

    fn u16(&self, offset: u64) -> Result<u16> {
        let mut cursor = Cursor::new_pos(&self.inner, offset);
        Ok(cursor.read_u16::<LittleEndian>()?)
    }

    fn u32(&self, offset: u64) -> Result<u32> {
        let mut cursor = Cursor::new_pos(&self.inner, offset);
        Ok(cursor.read_u32::<LittleEndian>()?)
    }

    fn u64_lohi(&self, lo: u64, hi: u64) -> Result<u64> {
        Ok(self.u32(lo)? as u64 + ((self.u32(hi)? as u64) << 32))
    }

    fn vec(&self, offset: u64, len: usize) -> Result<Vec<u8>> {
        let mut v = vec![0u8; len];
        self.inner.read_exact_at(offset, &mut v)?;
        Ok(v)
    }
}

fn main() -> Result<()> {
    // open our ext4 partition, READ-ONLY.
    let file = OpenOptions::new().read(true).open("/dev/sda3")?;

    let sb = Superblock::new(&file)?;

    let root_inode = InodeNumber(2).inode(&sb, &file)?;
    let root_inode_type = root_inode.filetype();
    println!("({root_inode_type:?}) {root_inode:#?}");

    let root_entries = root_inode.dir_entries(&sb, &file)?;
    println!("{root_entries:#?}");

    Ok(())
}

use hex_slice::AsHex;
use positioned_io::ReadAt;
use std::fs::OpenOptions;

fn main() -> Result<(), std::io::Error> {
    let file = OpenOptions::new().read(true).open("/dev/sda3")?;
    let mut buf = vec![0u8; 128];

    // read 128 bytes of the file starting at offset 1024
    file.read_exact_at(1024, &mut buf)?;

    println!("{:x}", buf.as_hex());

    Ok(())
}

# read-raw-ext4

Rust code sample to read an ext4 partition from Rust, for:

  * <https://fasterthanli.me/series/reading-files-the-hard-way/part-3>

## Usage

Don't.

This opens a raw ext4 partition, in read-only mode because I'm cautious, but
still, it needs to run as root and you should REALLY make sure it does what you
think it does before running it.

So:

  * **If you lose data by running this, I can't be held responsible**
  * This is only designed to work on Ext4 partitions
  * This was tested exactly twice (on a 2019 Manjaro install and a 2023 Ubuntu install)

With that disclaimer out of the way, here's how I run it.

First, we find out what our root partition is:

```shell
$ df -Th /             
Filesystem     Type  Size  Used Avail Use% Mounted on
/dev/sda3      ext4  548G   97G  424G  19% /
```

Mine's `/dev/sda3`, and it's `ext4`, so this tool might work. Time to build it:

```shell
$ cargo build --release
(cut)
```

And run it:

```shell
$ sudo RUST_BACKTRACE=1 ./target/release/read-raw-ext4 /dev/sda3
(Directory) Inode {
    mode: 40755,
    size: 4096,
}
(Directory) Inode {
    mode: 40755,
    size: 12288,
}
(Regular) Inode {
    mode: 100644,
    size: 220,
}
---------------------------------------------
127.0.0.1       localhost
127.0.1.1       sonic

# The following lines are desirable for IPv6 capable hosts
::1     ip6-localhost ip6-loopback
fe00::0 ip6-localnet
ff00::0 ip6-mcastprefix
ff02::1 ip6-allnodes
ff02::2 ip6-allrouters
```

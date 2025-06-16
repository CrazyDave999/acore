use acore_fs::{AcoreFileSystem, BlockDevice, BLOCK_SIZE};
use clap::{App, Arg};
use std::collections::HashSet;
use std::fs::{read_dir, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};

struct BlockFile(Mutex<File>);
impl BlockDevice for BlockFile {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SIZE) as u64))
            .expect("Error when seeking!");
        assert_eq!(
            file.read(buf).unwrap(),
            BLOCK_SIZE,
            "Not a complete block!, block_id: {}",
            block_id
        );
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SIZE) as u64))
            .expect("Error when seeking!");
        assert_eq!(
            file.write(buf).unwrap(),
            BLOCK_SIZE,
            "Not a complete block!"
        );
    }
}

fn main() {
    acore_fs_pack().expect("Error when packing AcoreFS!");
}

fn acore_fs_pack() -> std::io::Result<()> {
    println!(
        r"
 ________   ________   ________   ________   _______           ________  ________
|\   __  \ |\   ____\ |\   __  \ |\   __  \ |\  ___ \         |\  _____\|\   ____\
\ \  \|\  \\ \  \___| \ \  \|\  \\ \  \|\  \\ \   __/|        \ \  \__/ \ \  \___|_
 \ \   __  \\ \  \     \ \  \\\  \\ \   _  _\\ \  \_|/__       \ \   __\ \ \_____  \
  \ \  \ \  \\ \  \____ \ \  \\\  \\ \  \\  \|\ \  \_|\ \       \ \  \_|  \|____|\  \
   \ \__\ \__\\ \_______\\ \_______\\ \__\\ _\ \ \_______\       \ \__\     ____\_\  \
    \|__|\|__| \|_______| \|_______| \|__|\|__| \|_______|        \|__|    |\_________\
                                                                           \|_________|
    "
    );
    println!("AcoreFS packer started...");
    let matches = App::new("EasyFileSystem packer")
        .arg(
            Arg::with_name("source")
                .short("s")
                .long("source")
                .takes_value(true)
                .help("Executable source dir(with backslash)"),
        )
        .arg(
            Arg::with_name("target")
                .short("t")
                .long("target")
                .takes_value(true)
                .help("Executable target dir(with backslash)"),
        )
        .get_matches();
    let src_path = matches.value_of("source").unwrap();
    let target_path = matches.value_of("target").unwrap();
    println!("src_path = {}\ntarget_path = {}", src_path, target_path);
    let block_file = Arc::new(BlockFile(Mutex::new({
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!("{}{}", target_path, "fs.img"))?;
        f.set_len(16 * 2048 * BLOCK_SIZE as u64)?;
        f
    })));

    println!(
        "Successfully created fs.img with size: {} bytes",
        16 * 2048 * BLOCK_SIZE
    );

    let apps: Vec<_> = read_dir(src_path)?
        .into_iter()
        .map(|dir_entry| {
            let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
            name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
            name_with_ext
        })
        .collect();

    println!("Found {} apps to pack", apps.len());

    // 16MiB, at most 4096 inodes
    let afs = AcoreFileSystem::new(block_file, 16 * 2048, 4096);

    println!("Successfully created AcoreFileSystem");

    let root_inode = AcoreFileSystem::root_inode(afs.clone());

    println!("Successfully created root inode");

    // create /bin and /tests dir

    let bin_inode = root_inode
        .access_dir_entry("bin", acore_fs::DiskInodeType::Directory, true)
        .unwrap();
    let tests_inode = root_inode
        .access_dir_entry("tests", acore_fs::DiskInodeType::Directory, true)
        .unwrap();

    let bin_names = HashSet::from([
        "cat", "cd", "cp", "fstat", "ll", "ls", "mkdir", "shell", "init", "exit",
    ]);

    for app in apps {
        // load app data from host file system
        let mut host_file = File::open(format!("{}{}", target_path, app))?;
        let mut all_data: Vec<u8> = Vec::new();
        host_file.read_to_end(&mut all_data)?;
        // create a file in acore-fs

        let inode = if bin_names.contains(app.as_str()) {
            bin_inode
                .access_dir_entry(app.as_str(), acore_fs::DiskInodeType::File, true)
                .unwrap()
        } else {
            tests_inode
                .access_dir_entry(app.as_str(), acore_fs::DiskInodeType::File, true)
                .unwrap()
        };
        // write data to acore-fs
        println!("Processing file: {}, size: {} bytes", app, all_data.len());
        inode.write_at(0, all_data.as_slice());
    }
    Ok(())
}

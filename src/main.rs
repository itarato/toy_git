#[macro_use]
extern crate log;

use clap::Parser;
use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use sha1::{Digest, Sha1};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

use crate::common::bytes_to_string;

mod common;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    command: String,

    #[arg(short = 'p', long)]
    object_hash: Option<String>,

    #[arg(short = 'w', long)]
    file_path: Option<String>,
}

fn main() {
    // unsafe { std::env::set_var("RUST_LOG", "debug") };
    pretty_env_logger::init();

    let args = Args::parse();

    match args.command.as_str() {
        "init" => {
            fs::create_dir(".git").unwrap();
            fs::create_dir(".git/objects").unwrap();
            fs::create_dir(".git/refs").unwrap();
            fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
            info!("Initialized git directory")
        }

        "cat-file" => {
            let hash = args.object_hash.unwrap();
            let prefix = &hash[0..2];
            let filename = &hash[2..];
            let filepath_raw = format!(".git/objects/{}/{}", prefix, filename);
            let filepath = Path::new(&filepath_raw);
            let file = File::open(filepath).unwrap();
            let mut decoder = ZlibDecoder::new(file);
            let mut content_buf = vec![];
            decoder.read_to_end(&mut content_buf).unwrap();

            let mut content_start_index = 0;
            for i in 0..content_buf.len() {
                if content_buf[i] == 0 {
                    content_start_index = i + 1;
                }
            }

            let content = str::from_utf8(&content_buf[content_start_index..]).unwrap();
            print!("{}", content);
        }

        "hash-object" => {
            let mut content_suffix = fs::read_to_string(&args.file_path.unwrap())
                .unwrap()
                .as_bytes()
                .to_vec();
            let mut content = format!("blob {}\0", content_suffix.len())
                .as_bytes()
                .to_vec();
            content.append(&mut content_suffix);

            let mut hasher = Sha1::new();
            hasher.update(&content);
            let hash = hasher.finalize();
            let hash_str = bytes_to_string(&hash[..]);

            println!("{}", hash_str);

            let prefix = &hash_str[0..2];
            let filename = &hash_str[2..];
            let folderpath_raw = format!(".git/objects/{}", prefix);
            let folderpath = Path::new(&folderpath_raw);
            fs::create_dir_all(folderpath).unwrap();

            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&content).unwrap();
            let content_encoded = encoder.finish().unwrap();

            let filepath_raw = format!(".git/objects/{}/{}", prefix, filename);
            let filepath = Path::new(&filepath_raw);
            fs::write(filepath, content_encoded).unwrap();
        }

        other => {
            error!("unknown command: {}", other)
        }
    }
}

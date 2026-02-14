#[macro_use]
extern crate log;

use clap::Parser;
use flate2::{Compression, write::ZlibEncoder};
use sha1::{Digest, Sha1};
use std::{collections::BTreeMap, fs, io::Write};

use crate::common::{Entry, Hash, bytes_to_string, read_file_into_encoded_blob};

mod common;
mod reader;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    command: String,
    object_hash: Option<String>,

    #[arg(short = 'p', long)]
    cat_file_hash: Option<String>,

    #[arg(short = 'w', long)]
    file_path: Option<String>,

    #[arg(long = "name-only")]
    name_only: bool,
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

        "cat-file" => match Hash::new(args.cat_file_hash.unwrap()).read() {
            Entry::File { content } => print!("{}", content),
            Entry::Tree { .. } => unimplemented!(),
        },

        "hash-object" => {
            let hash = write_blob(&args.file_path.unwrap());
            println!("{}", hash.hash);
        }

        "ls-tree" => match Hash::new(args.object_hash.unwrap()).read() {
            Entry::File { .. } => unimplemented!(),
            Entry::Tree { entries } => {
                for entry in entries {
                    if args.name_only {
                        println!("{}", entry.filename);
                    } else {
                        println!(
                            "{} {} {}\t{}",
                            entry.perm,
                            entry.perm_to_string(),
                            entry.hash.hash,
                            entry.filename
                        );
                    }
                }
            }
        },

        "write-tree" => {
            let hash = write_tree("./");
            println!("{}", hash.hash);
        }

        other => {
            error!("unknown command: {}", other)
        }
    }
}

fn write_blob(file_path: &str) -> Hash {
    let content = read_file_into_encoded_blob(file_path);

    let mut hasher = Sha1::new();
    hasher.update(&content);
    let hash = Hash::new(bytes_to_string(&hasher.finalize()));

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&content).unwrap();
    let content_encoded = encoder.finish().unwrap();

    hash.write_content(&content_encoded[..]);

    hash
}

fn write_tree(dir: &str) -> Hash {
    let mut folder_entries: BTreeMap<String, Vec<u8>> = BTreeMap::new();

    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.file_name().unwrap().to_string_lossy() == ".git" {
            continue;
        }

        let metadata = fs::metadata(&path).unwrap();

        let mut bytes = vec![];

        let bytes = if metadata.is_dir() {
            let hash = write_tree(&path.to_string_lossy());

            bytes.extend_from_slice(b"40000 ");
            bytes.extend_from_slice(path.file_name().unwrap().to_string_lossy().as_bytes());
            bytes.push(0);
            bytes.extend_from_slice(&hash.as_bytes());
            bytes
        } else {
            let hash = write_blob(&path.to_string_lossy());

            bytes.extend_from_slice(b"100644 ");
            bytes.extend_from_slice(path.file_name().unwrap().to_string_lossy().as_bytes());
            bytes.push(0);
            bytes.extend_from_slice(&hash.as_bytes());
            bytes
        };

        folder_entries.insert(
            path.file_name().unwrap().to_string_lossy().to_string(),
            bytes,
        );
    }

    let mut entries = folder_entries
        .into_values()
        .flat_map(|e| e)
        .collect::<Vec<_>>();

    let mut bytes = format!("tree {}\0", entries.len()).as_bytes().to_vec();
    bytes.append(&mut entries);

    let mut hasher = Sha1::new();
    hasher.update(&bytes);
    let hash = Hash::new(bytes_to_string(&hasher.finalize()));

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&bytes).unwrap();
    let content_encoded = encoder.finish().unwrap();

    hash.write_content(&content_encoded[..]);

    hash
}

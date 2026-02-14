#[macro_use]
extern crate log;

use clap::Parser;
use flate2::{Compression, write::ZlibEncoder};
use sha1::{Digest, Sha1};
use std::{fs, io::Write};

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
            let content = read_file_into_encoded_blob(&args.file_path.unwrap());

            let mut hasher = Sha1::new();
            hasher.update(&content);
            let hash = Hash::new(bytes_to_string(&hasher.finalize()));

            println!("{}", hash.hash);

            fs::create_dir_all(hash.folder_path()).unwrap();

            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&content).unwrap();
            let content_encoded = encoder.finish().unwrap();

            fs::write(hash.file_path(), content_encoded).unwrap();
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

        other => {
            error!("unknown command: {}", other)
        }
    }
}

#[macro_use]
extern crate log;

use clap::{Parser, Subcommand};
use flate2::{Compression, write::ZlibEncoder};
use sha1::{Digest, Sha1};
use std::{collections::BTreeMap, fs, io::Write};

use crate::common::{
    Entry, Hash, bytes_to_string, hex_len_prefixed_string, read_file_into_encoded_blob,
};

mod common;
mod reader;

#[derive(Subcommand)]
enum CliCommand {
    Init,
    CatFile {
        #[arg(short = 'p', long)]
        parent_hash: String,
    },
    HashObject {
        #[arg(short = 'w', long)]
        file_path: String,
    },
    LsTree {
        object_hash: String,

        #[arg(long = "name-only")]
        name_only: bool,
    },
    WriteTree,
    CommitTree {
        tree_hash: String,

        #[arg(short = 'p', long)]
        parent_hash: String,

        #[arg(short, long)]
        message: String,
    },
    Clone {
        url: String,
        dir: String,
    },
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Args {
    #[command(subcommand)]
    command: CliCommand,
}

fn main() {
    // unsafe { std::env::set_var("RUST_LOG", "debug") };
    pretty_env_logger::init();

    let args = Args::parse();

    match args.command {
        CliCommand::Init => {
            fs::create_dir(".git").unwrap();
            fs::create_dir(".git/objects").unwrap();
            fs::create_dir(".git/refs").unwrap();
            fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
            info!("Initialized git directory")
        }

        CliCommand::CatFile { parent_hash } => match Hash::new(parent_hash).read() {
            Entry::File { content } => print!("{}", content),
            Entry::Tree { .. } => unimplemented!(),
        },

        CliCommand::HashObject { file_path } => {
            let hash = write_blob(&file_path);
            println!("{}", hash.hash);
        }

        CliCommand::LsTree {
            object_hash,
            name_only,
        } => match Hash::new(object_hash).read() {
            Entry::File { .. } => unimplemented!(),
            Entry::Tree { entries } => {
                for entry in entries {
                    if name_only {
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

        CliCommand::WriteTree => {
            let hash = write_tree("./");
            println!("{}", hash.hash);
        }

        CliCommand::CommitTree {
            parent_hash,
            message,
            ..
        } => {
            // commit <size>\0tree <tree_sha>
            // parent <parent_sha>
            // author <name> <<email>> <timestamp> <timezone>
            // committer <name> <<email>> <timestamp> <timezone>

            // <commit message>

            let mut suffix = vec![];

            let tree_hash = write_tree("./");
            suffix.extend_from_slice(b"tree ");
            suffix.extend_from_slice(tree_hash.hash.as_bytes());
            suffix.push(b'\n');

            suffix.extend_from_slice(b"parent ");
            suffix.extend_from_slice(parent_hash.as_bytes());
            suffix.push(b'\n');

            suffix.extend_from_slice(b"author John Doe <john@example.com> 1234567890 +0000\n");
            suffix.extend_from_slice(b"committer John Doe <john@example.com> 1234567890 +0000\n\n");

            suffix.extend_from_slice(message.as_bytes());
            suffix.push(b'\n');

            let mut content = format!("commit {}\0", suffix.len()).as_bytes().to_vec();

            content.append(&mut suffix);

            let hash = write_payload(content);
            println!("{}", hash.hash);
        }

        CliCommand::Clone { url, dir } => {
            let get_head_sha_url = format!(
                "{}{}",
                url.trim_end_matches('/'),
                "/info/refs?service=git-upload-pack"
            );
            let response = reqwest::blocking::get(get_head_sha_url).unwrap();
            let response_body = response.text().unwrap();

            debug!("SHA reponse body: {}", response_body);

            let lines = response_body.lines().collect::<Vec<_>>();
            let sha1_head_str = lines[1][8..48].to_string();
            debug!("Clone sha1_head: {}", sha1_head_str);

            let want_content = format!(
                "want {} multi_ack_detailed thin-pack side-band-64k ofs-delta\n",
                sha1_head_str
            );
            let want_payload = format!("{}00000009done", hex_len_prefixed_string(&want_content));

            debug!("Request payload: {}", want_payload);

            let want_url = format!("{}{}", url.trim_end_matches('/'), "/git-upload-pack");
            let client = reqwest::blocking::Client::new();
            let response = client
                .post(&want_url)
                .header("Content-Type", "application/x-git-upload-pack-request")
                .body(want_payload)
                .send()
                .unwrap();
            debug!("Response status: {:?}", response.status());
            debug!("Response headers: {:?}", response.headers());
            let response_body = response.text().unwrap();

            debug!("Clone POST response: {}", response_body);

            unimplemented!()
        }
    }
}

fn write_payload(payload: Vec<u8>) -> Hash {
    let mut hasher = Sha1::new();
    hasher.update(&payload);
    let hash = Hash::new(bytes_to_string(&hasher.finalize()));

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&payload).unwrap();
    let content_encoded = encoder.finish().unwrap();

    hash.write_content(&content_encoded[..]);

    hash
}

fn write_blob(file_path: &str) -> Hash {
    write_payload(read_file_into_encoded_blob(file_path))
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

    write_payload(bytes)
}

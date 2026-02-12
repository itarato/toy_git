#[macro_use]
extern crate log;

use clap::Parser;
use flate2::read::ZlibDecoder;
use std::{
    fs::{self, File},
    io::Read,
    path::Path,
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    command: String,

    #[arg(short = 'p', long)]
    object_hash: Option<String>,
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
        other => {
            error!("unknown command: {}", other)
        }
    }
}

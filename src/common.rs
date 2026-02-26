use std::{
    fs::{self, File},
    io::{Read, Write},
};

use flate2::read::ZlibDecoder;

use crate::reader::Reader;

pub(crate) fn bytes_to_string(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join("")
}

pub(crate) fn debug_byte_str(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn hex_len_prefixed_string(s: &str) -> String {
    format!("{:04x}{}", s.len() + 4, s)
}

#[derive(Debug)]
pub(crate) struct Hash {
    pub(crate) hash: String,
}

impl Hash {
    pub(crate) fn new(hash: String) -> Self {
        Self { hash }
    }

    pub(crate) fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];

        let mut i = 0;
        while i < self.hash.len() {
            bytes.push(u8::from_str_radix(&self.hash[i..i + 2], 16).unwrap());
            i += 2;
        }

        bytes
    }

    fn folder_path(&self) -> String {
        let prefix = &self.hash[0..2];
        format!(".git/objects/{}", prefix)
    }

    fn ensure_folder_exist(&self) {
        std::fs::create_dir_all(&self.folder_path()).unwrap();
    }

    pub(crate) fn file_path(&self) -> String {
        let prefix = &self.hash[0..2];
        let filename = &self.hash[2..];
        format!(".git/objects/{}/{}", prefix, filename)
    }

    pub(crate) fn write_content(&self, content: &[u8]) {
        self.ensure_folder_exist();
        let mut file = File::create(self.file_path()).unwrap();
        file.write_all(content).unwrap();
    }

    pub(crate) fn read(&self) -> Entry {
        let file = File::open(self.file_path()).unwrap();
        let mut decoder = ZlibDecoder::new(file);
        let mut content_buf = vec![];
        decoder.read_to_end(&mut content_buf).unwrap();

        let mut reader = Reader::new(&content_buf[..]);
        let kind = str::from_utf8(reader.pop_while(|c| c != &b' ')).unwrap();

        reader.pop(); // space
        let _payload_len =
            usize::from_str_radix(str::from_utf8(reader.pop_while(|c| c != &0)).unwrap(), 10)
                .unwrap();
        reader.pop(); // \0

        match kind {
            "blob" => {
                let content = str::from_utf8(reader.pop_all()).unwrap().to_string();
                Entry::File { content }
            }
            "tree" => {
                let mut entries = vec![];

                while !reader.is_empty() {
                    // tree <size>\0
                    // <mode> <name>\0<20_byte_sha>
                    // <mode> <name>\0<20_byte_sha>
                    let perm = str::from_utf8(reader.pop_while(|c| c != &b' '))
                        .unwrap()
                        .to_string();
                    reader.pop(); // space
                    let filename = str::from_utf8(reader.pop_while(|c| c != &0))
                        .unwrap()
                        .to_string();
                    reader.pop(); // \0
                    let hash_bytes = reader.popn(20);
                    let hash_str = bytes_to_string(hash_bytes);
                    let hash = Hash::new(hash_str);

                    entries.push(TreeEntry {
                        perm,
                        filename,
                        hash,
                    });
                }

                Entry::Tree { entries }
            }
            other => {
                error!("Unrecognized entry indicator: {}", other);
                panic!()
            }
        }
    }
}

pub(crate) fn read_file_into_encoded_blob(file_path: &str) -> Vec<u8> {
    let mut file = fs::File::open(file_path).unwrap();
    let mut content_suffix = vec![];
    file.read_to_end(&mut content_suffix).unwrap();
    let mut content = format!("blob {}", content_suffix.len()).as_bytes().to_vec();
    content.push(0);
    content.append(&mut content_suffix);

    content
}

#[derive(Debug)]
pub(crate) struct TreeEntry {
    pub(crate) perm: String,
    pub(crate) filename: String,
    pub(crate) hash: Hash,
}

impl TreeEntry {
    pub(crate) fn perm_to_string(&self) -> &str {
        match self.perm.as_str() {
            "100644" => "blob",
            "040000" => "tree",
            other => {
                error!("Perm type not implemented: {}", other);
                panic!()
            }
        }
    }
}

pub(crate) enum Entry {
    Tree { entries: Vec<TreeEntry> },
    File { content: String },
}

#[cfg(test)]
mod test {
    use crate::common::{bytes_to_string, hex_len_prefixed_string};

    #[test]
    fn test_bytes_to_string() {
        assert_eq!("0314a3", bytes_to_string(&[0x03, 0x14, 0xa3]));
    }

    #[test]
    fn test() {
        assert_eq!("000a123456", hex_len_prefixed_string("123456"));
    }
}

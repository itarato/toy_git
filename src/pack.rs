use flate2::read::ZlibDecoder;
use std::{
    collections::{BTreeMap, HashMap},
    io::Read,
};

pub(crate) enum PackObjectType {
    Commit,
    Tree,
    Blob,
    OffsetDelta,
}

pub(crate) struct PackObject {
    pub(crate) kind: PackObjectType,
    pub(crate) compressed_payload: Vec<u8>,
    pub(crate) decompressed_payload: Vec<u8>,
}

pub(crate) struct PackReader<'a> {
    slice: &'a [u8],
    slice_ptr: usize,
}

impl<'a> PackReader<'a> {
    pub(crate) fn new(slice: &'a [u8]) -> Self {
        Self {
            slice,
            slice_ptr: 0,
        }
    }

    pub fn read(mut self) {
        let mut objects = BTreeMap::new();

        let pack_marker = self.popn(4);
        let pack_version = self.popn(4);
        let pack_object_count_bytes = self.popn(4);
        let pack_object_count =
            u32::from_be_bytes(pack_object_count_bytes[..].try_into().unwrap()) as usize;

        debug!("Marker: {:?}", pack_marker);
        debug!("Version: {:?}", pack_version);
        debug!("Object Count: {:?}", pack_object_count);
        debug!("Pack Payload Size: {:?}", self.slice.len());

        for _ in 0..pack_object_count {
            debug!("At offset (pre meta): {}", self.slice_ptr);
            let object_location = self.slice_ptr;
            let object_type = (self.slice[0] >> 4) & 0b111;
            let object_decompressed_size = self.read_varint(0b1000_1111);
            debug!("Payload decompressed size: {}", object_decompressed_size);
            debug!("Object type: {}", object_type);
            debug!("At offset (post meta): {}", self.slice_ptr);

            match object_type {
                1 => {
                    // Commit
                    let (decoded, encoded_len) = self.decode_current();
                    objects.insert(
                        object_location,
                        PackObject {
                            kind: PackObjectType::Commit,
                            compressed_payload: self.slice[..encoded_len].to_vec(),
                            decompressed_payload: decoded,
                        },
                    );

                    self.dropn(encoded_len);
                }
                2 => {
                    // Tree
                    let (decoded, encoded_len) = self.decode_current();
                    objects.insert(
                        object_location,
                        PackObject {
                            kind: PackObjectType::Tree,
                            compressed_payload: self.slice[..encoded_len].to_vec(),
                            decompressed_payload: decoded,
                        },
                    );

                    self.dropn(encoded_len);
                }
                3 => {
                    // Tree
                    let (decoded, encoded_len) = self.decode_current();
                    objects.insert(
                        object_location,
                        PackObject {
                            kind: PackObjectType::Blob,
                            compressed_payload: self.slice[..encoded_len].to_vec(),
                            decompressed_payload: decoded,
                        },
                    );

                    self.dropn(encoded_len);
                }
                6 => {
                    // OFS_DELTA
                    let offset = self.read_offset_varint();
                    let base_object = objects.get(&(object_location - offset)).unwrap();
                    debug!("OFS_DELTA offset: {}", offset);

                    let (decoded, encoded_len) = self.decode_current();
                    // debug!("OFS_DELTA decoded: {:?}", String::from_utf8(decoded));

                    objects.insert(
                        object_location,
                        PackObject {
                            kind: PackObjectType::OffsetDelta,
                            compressed_payload: vec![],
                            decompressed_payload: vec![],
                        },
                    );

                    self.dropn(encoded_len);
                }
                other => {
                    error!("Unknown object type: {}", other);
                    panic!()
                }
            };
        }
    }

    fn decode_current(&self) -> (Vec<u8>, usize) {
        let mut decoder = ZlibDecoder::new(self.slice);
        let mut content_buf = vec![];
        decoder.read_to_end(&mut content_buf).unwrap();

        (content_buf, decoder.total_in() as usize)
    }

    fn popn(&mut self, n: usize) -> Vec<u8> {
        let out = &self.slice[0..n];
        self.slice = &self.slice[n..];
        self.slice_ptr += n;
        out.to_vec()
    }

    fn dropn(&mut self, n: usize) {
        self.slice = &self.slice[n..];
        self.slice_ptr += n;
    }

    fn read_varint(&mut self, mut first_byte_mask: u8) -> usize {
        let mut out = 0;

        loop {
            let byte = self.slice[0] & first_byte_mask;
            first_byte_mask = 0xFF;

            self.dropn(1);

            out <<= 7;
            out |= (byte & 0b0111_1111) as usize;

            if byte & 0b1000_0000 == 0 {
                break;
            }
        }

        out
    }

    fn read_offset_varint(&mut self) -> usize {
        let mut byte = self.popn(1)[0];
        let mut out = (byte & 0b0111_1111) as usize;

        while byte & 0b1000_0000 > 0 {
            byte = self.popn(1)[0];
            out += 1;
            out <<= 7;
            out |= (byte & 0b0111_1111) as usize;
        }

        out
    }
}

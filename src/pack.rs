pub(crate) struct PackReader<'a> {
    slice: &'a [u8],
}

impl<'a> PackReader<'a> {
    pub(crate) fn new(slice: &'a [u8]) -> Self {
        Self { slice }
    }

    pub fn read(mut self) {
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
            let object_type = (self.slice[0] >> 4) & 0b111;
            let object_size = self.read_varint(0b1000_1111);
            debug!("Payload size: {}", object_size);

            let _payload = self.popn(object_size);

            debug!("Object type: {}", object_type);
        }
    }

    fn popn(&mut self, n: usize) -> Vec<u8> {
        let out = &self.slice[0..n];
        self.slice = &self.slice[n..];
        out.to_vec()
    }

    fn read_varint(&mut self, mut first_byte_mask: u8) -> usize {
        let mut out = 0;

        loop {
            let byte = self.slice[0] & first_byte_mask;
            first_byte_mask = 0xFF;

            self.slice = &self.slice[1..];

            out <<= 7;
            out |= (byte & 0b0111_1111) as usize;

            if byte & 0b1000_0000 == 0 {
                break;
            }
        }

        out
    }
}

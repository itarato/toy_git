pub(crate) struct Reader<'a, T> {
    stream: &'a [T],
}

impl<'a, T> Reader<'a, T> {
    pub(crate) fn new(stream: &'a [T]) -> Self {
        Self { stream }
    }

    pub(crate) fn pop(&mut self) -> &'a T {
        let out = &self.stream[0];
        self.stream = &self.stream[1..];
        out
    }

    pub(crate) fn popn(&mut self, n: usize) -> &'a [T] {
        let out = &self.stream[..n];
        self.stream = &self.stream[n..];
        out
    }

    pub(crate) fn pop_all(&mut self) -> &'a [T] {
        let out = &self.stream[..];
        self.stream = &self.stream[self.stream.len()..];
        out
    }

    pub(crate) fn pop_while<F>(&mut self, pred: F) -> &'a [T]
    where
        F: Fn(&T) -> bool,
    {
        let mut len = 0usize;

        for i in 0..self.stream.len() {
            if !pred(&self.stream[i]) {
                break;
            }

            len += 1;
        }

        let out = &self.stream[..len];
        self.stream = &self.stream[len..];
        out
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.stream.is_empty()
    }
}

impl<'a> Reader<'a, u8> {
    pub(crate) fn pop_varint(&mut self) -> u64 {
        let mut result = 0u64;

        loop {
            result <<= 7;
            let byte = self.pop();
            result |= (byte & 0b0111_1111) as u64;

            if byte & 0b1000_0000 == 0 {
                break;
            }
        }

        result
    }
}

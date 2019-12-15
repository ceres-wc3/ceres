#[derive(Debug)]
pub struct Lines<'src> {
    source:      &'src [u8],
    current_pos: usize,
}

impl<'src> Lines<'src> {
    pub fn new(source: &'src [u8]) -> Lines<'src> {
        Lines {
            source,
            current_pos: 0,
        }
    }
}

impl<'src> Iterator for Lines<'src> {
    type Item = &'src [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let mut start = self.current_pos;
        while start < self.source.len()
            && (self.source[start] == b'\r' || self.source[start] == b'\n')
        {
            start += 1;
        }

        let mut end = start;
        while end < self.source.len() && !(self.source[end] == b'\r' || self.source[end] == b'\n') {
            end += 1;
        }

        self.current_pos = end;

        if end > start {
            Some(&self.source[start..end])
        } else {
            None
        }
    }
}

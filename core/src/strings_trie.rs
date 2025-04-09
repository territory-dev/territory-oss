use std::cmp::min;

pub type TrieValue = u64;


const CODE_BACKSPACE: u8 = 1;
const CODE_LEAF: u8 = 0;

pub enum TrieSymbol {
    EOF,
    ASCIIChar(u8),
    Backspace(u8),
    Leaf(TrieValue),
}


pub struct TrieWriter {
    data: Vec<u8>,
    key: Vec<u8>,
}


impl TrieWriter {
    pub fn new() -> Self {
        Self { data: Vec::new(), key: Vec::new() }
    }

    pub fn push(&mut self, key_str: &str, value: TrieValue) {
        let key: &[u8] = key_str.as_ref();
        let mut common_prefix_len = 0;
        while common_prefix_len < key.len() &&
              common_prefix_len < self.key.len() &&
              key[common_prefix_len] == self.key[common_prefix_len]
        {
            common_prefix_len += 1;
        }

        let mut shift: usize = 0;
        while self.key.len() > common_prefix_len {
            shift += 1;
            self.key.pop();
        }
        while shift > 0 {
            let ds: u8 = min(shift, 0b1111).try_into().unwrap();
            self.data.push(1 << 7 | CODE_BACKSPACE << 4 | ds);
            shift -= ds as usize;
        }

        for c in &key[common_prefix_len..] {
            if *c < 32 {
                println!("skipping control character {} in key {:?}", c, key_str);
                continue;
            }
            if *c > 127 {
                println!("skipping non-ASCII character {} in key {:?}", c, key_str);
                continue;
            }
            self.key.push(*c);
            self.data.push(*c);
        }

        let value_bytes = value.to_be_bytes();
        let mut nz: u8 = 8;
        for i in 0..8u8 {
            if value_bytes[i as usize] != 0 {
                nz = i;
                break;
            }
        }
        self.data.push(1 << 7 | CODE_LEAF << 4 | (8-nz));
        self.data.extend_from_slice(&value_bytes[nz as usize..8]);
    }

    pub fn data(self) -> Vec<u8> {
        let mut d = self.data;
        d.shrink_to_fit();
        d
    }
}

pub struct TrieReader<'a> {
    data: &'a Vec<u8>,
    offset: usize,
}

#[derive(Debug)]
pub enum TrieError {
    UnexpectedEOF,
    UnknownCode { code: u8, offset: usize },
}

impl<'a> TrieReader<'a> {
    pub fn new(data: &'a Vec<u8>) -> Self {
        Self { data, offset: 0 }
    }

    pub fn read_symbol(&mut self) -> Result<TrieSymbol, TrieError> {
        use TrieSymbol::*;

        if self.offset >= self.data.len() {
            return Ok(EOF);
        }

        let byte = self.data[self.offset];
        self.offset += 1;

        if byte <= 127 {
            return Ok(ASCIIChar(byte));
        } else {
            let code = (byte >> 4) & 0b111;
            let shift = byte & 0b1111;
            match code {
                CODE_BACKSPACE => return Ok(Backspace(shift)),
                CODE_LEAF => {
                    let mut value: u64 = 0;
                    for _ in 0..shift {
                        let Some(vbyte) = self.data.get(self.offset) else {
                            return Err(TrieError::UnexpectedEOF);
                        };
                        value = (value << 8) | (*vbyte as u64);
                        self.offset += 1;
                    }
                    return Ok(Leaf(value));
                }
                _ => {
                    return Err(TrieError::UnknownCode { code: byte, offset: self.offset-1 });
                }
            }
        }
    }

    pub fn items(self) -> TrieItems<'a> {
        TrieItems { reader: self, key: String::new() }
    }
}

pub struct TrieItems<'a> {
    reader: TrieReader<'a>,
    key: String,
}


impl<'a> Iterator for TrieItems<'a> {
    type Item = (String, TrieValue);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.reader.read_symbol() {
                Err(e) => { panic!("trie read error: {:?}", e); },
                Ok(TrieSymbol::EOF) => { return None; },
                Ok(TrieSymbol::ASCIIChar(c)) => {
                    self.key.push(c as char);
                }
                Ok(TrieSymbol::Backspace(n)) => {
                    for _ in 0..n { self.key.pop(); }
                }
                Ok(TrieSymbol::Leaf(value)) => {
                    return Some((self.key.clone(), value));
                }
            }
        }
    }
}


#[cfg(test)]
mod test {
    use super::{TrieWriter, TrieReader};

    #[test]
    fn items() {
        let data = vec![
            ("a".to_string(), 1),
            ("aaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(), 2),
            ("aab".to_string(), 3),
            ("aba".to_string(), (1 << 31) | (1 << 20) | (1 << 15)),
            ("aba".to_string(), 4),
            ("abbc".to_string(), 5),
            ("baa".to_string(), 6),
        ];

        let mut w = TrieWriter::new();
        for (k, v) in &data { w.push(&k, *v); }

        let bytes = w.data();
        println!("bytes: {:?}", bytes);
        let decoded = TrieReader::new(&bytes)
            .items()
            .collect::<Vec<_>>();
        assert_eq!(data, decoded);
    }
}

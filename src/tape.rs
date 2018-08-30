use std::io::{self, Read, BufRead, BufReader, Cursor};
use std::fs::File;
use std::path::Path;

pub struct Tape {
    pub data: Vec<Vec<u8>>
}

impl Tape {
    pub fn new(tap: Vec<u8>) -> io::Result<Self> {
        let mut data = Vec::new();
        let mut tap = Cursor::new(tap);

        loop {
            let mut len = [0; 2];
            match tap.read_exact(&mut len) {
                Ok(_) => (),
                Err(_) => break,
            }
            let len = len[0] as usize | ((len[1] as usize) << 8);
            let mut block = vec![0; len];
            tap.read_exact(&mut block)?;
            data.push(block);
        }

        Ok(Tape{ data } )
    }
}

use std::fs::File;
use std::io::Write;

/*
 Simple reusable implementation of bufwriter to avoid repeated buffer allocations
 when writing to many different files
*/

pub struct ReusableWriter {
    buf: Vec<u8>,
    output_file: Option<File>,
}

impl ReusableWriter {
    pub fn new() -> Self {
        ReusableWriter {
            buf: Vec::with_capacity(8192000),
            output_file: None,
        }
    }

    pub fn change_file(&mut self, file: File) {
        self.output_file = Some(file);
    }

    pub fn write(&mut self, buf: &[u8]) {
        self.buf.write_all(buf).unwrap();
    }

    pub fn flush(&mut self) {
        self.output_file
            .as_mut().expect("attempted to flush without setting output_file")
            .write_all(&self.buf)
            .expect("failed to write to field store");
        self.buf.clear();
    }
}

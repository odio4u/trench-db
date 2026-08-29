
use std::fs::File;
use std::io::BufWriter;
use std::sync::{Arc, Mutex};

pub struct WALWriter {
    writer: Arc<Mutex<BufWriter<File>>>,
    position: u64,
    pathbuf: String,
}

impl WALWriter {
    
}
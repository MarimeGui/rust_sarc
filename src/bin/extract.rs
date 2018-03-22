extern crate sarc;
extern crate yaz0lib_rust;

use sarc::SARC;
use std::env;
use std::path::Path;
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() < 3 {
        let exec_name = args[0].to_string();
        println!("Usage: ./{} input_file output_folder", Path::new(&exec_name).file_name().unwrap().to_str().unwrap());
    } else if args.len() > 3 {
        println!("Please only give two arguments");
    } else {
        let input_file_path = Path::new(&args[1]);
        let output_folder_path = Path::new(&args[2]);
        let mut input_file_buf_reader = BufReader::new(File::open(input_file_path).expect("Failed to open file for reading"));
        let mut yaz_check_buffer = [0u8; 4];
        input_file_buf_reader.read_exact(&mut yaz_check_buffer).expect("Failed to read first Magic Number");
        input_file_buf_reader.seek(SeekFrom::Start(0)).expect("Failed to re-seek to beginning of the file");
        let mut sarc_cursor = if yaz_check_buffer == [b'Y', b'a', b'z', b'0'] {
            Cursor::new(yaz0lib_rust::decompress(&mut input_file_buf_reader).expect("Failed to decompress"))
        } else {
            let mut data = Vec::new();
            input_file_buf_reader.read_to_end(&mut data).expect("Failed to read all data");
            Cursor::new(data)
        };
        let sarc = SARC::import(&mut sarc_cursor).expect("Failed to read SARC");
        println!("Read SARC Successfully !");
        let files = sarc.get_files(&mut sarc_cursor).expect("Failed to read files");
        for file in files {
            println!("{}", file.name);
            file.export(output_folder_path).expect("Filed to write file");
        }
    }
}
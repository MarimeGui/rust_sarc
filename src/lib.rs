extern crate ez_io;
extern crate yaz0lib_rust;
#[macro_use] extern crate enum_primitive;

use ez_io::ReadE;
use enum_primitive::FromPrimitive;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::fs::File;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
struct WrongMagicNumber<T: PartialEq + Sized> {
    left: T,
    right: T
}

impl <T: PartialEq + Sized + fmt::Debug> Error for WrongMagicNumber<T> {
    fn description(&self) -> &str {
        "A Magic Number check Failed"
    }
}

impl <T: PartialEq + Sized + fmt::Debug> fmt::Display for WrongMagicNumber<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Incorrect Magic Number: {:?} != {:?}", self.left, self.right)
    }
}

#[derive(Debug)]
struct NotInEnum<T: PartialEq + Sized>  {
    value: T
}

impl <T: PartialEq + Sized + fmt::Debug> Error for NotInEnum<T> {
    fn description(&self) -> &str {
        "A value did not match anything in an enum"
    }
}

impl <T: PartialEq + Sized + fmt::Debug> fmt::Display for NotInEnum<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Not in the enum: {:?}", self.value)
    }
}

#[derive(Debug)]
struct NodeNameLengthMismatch {
    node_count: usize,
    name_count: usize
}

impl Error for NodeNameLengthMismatch {
    fn description(&self) -> &str {
        "There are not the same amount of nodes and names in file"
    }
}

impl fmt::Display for NodeNameLengthMismatch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} nodes, {:?} names", self.node_count, self.name_count)
    }
}


fn check_magic_number<T: PartialEq + Sized + fmt::Debug>(left: T, right: T) -> Result<(), WrongMagicNumber<T>> {
    if left != right {
        Err(WrongMagicNumber {
            left,
            right
        })
    } else {
        Ok(())
    }
}

fn smart_align_4<S: Seek>(seeker: &mut S) -> Result<(), Box<Error>> {
    let pos = seeker.seek(SeekFrom::Current(0))?;
    if pos % 4 != 0 {
        seeker.seek(SeekFrom::Current((4 - (pos % 4)) as i64))?;
    }
    Ok(())
}

fn read_text_entry<R: Read + Seek>(reader: &mut R) -> Result<String, Box<Error>> {
    let mut bytes: Vec<u8> = Vec::new();
    let mut current_byte: [u8; 1] = [0u8; 1];
    loop {
        reader.read_exact(&mut current_byte)?;
        if current_byte[0] == 0u8 {
            break
        } else {
            bytes.push(current_byte[0]);
        }
    }
    Ok(String::from_utf8(bytes)?)
}

pub struct SARC {
    pub header: SARCHeader,
    pub file_table: SFAT,
    pub name_table: SFNT
}

pub struct SARCHeader {
    pub header_length: u16,
    pub bom: ByteOrder,
    pub file_size: u32,
    pub data_offset: u32,
    pub version: u16
}

enum_from_primitive! {
    pub enum ByteOrder {
        Big = 0xFEFF,
        Little = 0xFFFE
    }
}

pub struct SFAT {
    pub header: SFATHeader,
    pub nodes: Vec<SFATNode>
}

pub struct SFATHeader {
    pub header_length: u16,
    pub node_count: u16,
    pub hash_multiplier: u32
}

pub struct SFATNode {
    pub file_name_hash: u32,
    pub file_attributes: u32,
    pub data_start_offset: u32,
    pub data_end_offset: u32
}

pub struct SFNT {
    pub header: SFNTHeader,
    pub file_names: Vec<String>
}

pub struct SFNTHeader {
    pub header_length: u16
}

pub struct SARCOutputFile {
    pub name: String,
    pub data: Vec<u8>
}

impl SARC {
    pub fn import<R: Read + Seek>(reader: &mut R) -> Result<SARC, Box<Error>> {
        let header = SARCHeader::import(reader)?;
        let file_table = SFAT::import(reader)?;
        let name_table = SFNT::import(reader, file_table.header.node_count)?;
        Ok(SARC {
            header,
            file_table,
            name_table
        })
    }
    pub fn get_files<R: Read + Seek>(&self, reader: &mut R) -> Result<Vec<SARCOutputFile>, Box<Error>> {
        if self.file_table.nodes.len() != self.name_table.file_names.len() {
            return Err(Box::new(NodeNameLengthMismatch {
                node_count: self.file_table.nodes.len(),
                name_count: self.name_table.file_names.len()
            }));
        };
        let mut out = Vec::new();
        for i in 0..self.file_table.nodes.len() {
            let name = self.name_table.file_names[i].clone();
            let data_start = self.header.data_offset + self.file_table.nodes[i].data_start_offset;
            let data_end = self.header.data_offset + self.file_table.nodes[i].data_end_offset;
            let data_length = self.file_table.nodes[i].data_end_offset - self.file_table.nodes[i].data_start_offset;
            let mut data = Vec::with_capacity(data_length as usize);
            reader.seek(SeekFrom::Start(u64::from(data_start)))?;
            while reader.seek(SeekFrom::Current(0))? != u64::from(data_end) {
                let mut buf = [0u8];
                reader.read_exact(&mut buf)?;
                data.push(buf[0]);
            }
            out.push(SARCOutputFile {
                name,
                data
            });
        }
        Ok(out)
    }
}

impl SARCHeader {
    pub fn import<R: Read + Seek>(reader: &mut R) -> Result<SARCHeader, Box<Error>> {
        let mut magic_number = [0u8; 4];
        reader.read_exact(&mut magic_number)?;
        check_magic_number(magic_number, [b'S', b'A', b'R', b'C'])?;
        let header_length = reader.read_be_to_u16()?;
        let bom_val = reader.read_be_to_u16()?;
        let bom = ByteOrder::from_u16(bom_val).ok_or(NotInEnum {value: bom_val})?;
        let file_size = reader.read_be_to_u32()?;
        let data_offset = reader.read_be_to_u32()?;
        let version = reader.read_be_to_u16()?;
        reader.seek(SeekFrom::Current(2))?;
        Ok(SARCHeader {
            header_length,
            bom,
            file_size,
            data_offset,
            version
        })
    }
}

impl SFAT {
    pub fn import<R: Read + Seek>(reader: &mut R) -> Result<SFAT, Box<Error>> {
        let header = SFATHeader::import(reader)?;
        let mut nodes = Vec::with_capacity(header.node_count as usize);
        while nodes.len() < header.node_count as usize {
            nodes.push(SFATNode::import(reader)?);
        }
        Ok(SFAT {
            header,
            nodes
        })
    }
}

impl SFATHeader {
    pub fn import<R: Read + Seek>(reader: &mut R) -> Result<SFATHeader, Box<Error>> {
        let mut magic_number = [0u8; 4];
        reader.read_exact(&mut magic_number)?;
        check_magic_number(magic_number, [b'S', b'F', b'A', b'T'])?;
        let header_length = reader.read_be_to_u16()?;
        let node_count = reader.read_be_to_u16()?;
        let hash_multiplier = reader.read_be_to_u32()?;
        Ok(SFATHeader {
            header_length,
            node_count,
            hash_multiplier
        })
    }
}

impl SFATNode {
    pub fn import<R: Read + Seek>(reader: &mut R) -> Result<SFATNode, Box<Error>> {
        let file_name_hash = reader.read_be_to_u32()?;
        let file_attributes = reader.read_be_to_u32()?;
        let data_start_offset = reader.read_be_to_u32()?;
        let data_end_offset = reader.read_be_to_u32()?;
        Ok(SFATNode {
            file_name_hash,
            file_attributes,
            data_start_offset,
            data_end_offset
        })
    }
}

impl SFNT {
    pub fn import<R: Read + Seek>(reader: &mut R, count: u16) -> Result<SFNT, Box<Error>> {
        let header = SFNTHeader::import(reader)?;
        let mut file_names = Vec::with_capacity(count as usize);
        while file_names.len() < count as usize {
            smart_align_4(reader)?;
            file_names.push(read_text_entry(reader)?);
        }
        Ok(SFNT {
            header,
            file_names
        })
    }
}

impl SFNTHeader {
    pub fn import<R: Read + Seek>(reader: &mut R) -> Result<SFNTHeader, Box<Error>> {
        let mut magic_number = [0u8; 4];
        reader.read_exact(&mut magic_number)?;
        check_magic_number(magic_number, [b'S', b'F', b'N', b'T'])?;
        let header_length = reader.read_be_to_u16()?;
        Ok(SFNTHeader {
            header_length
        })
    }
}

impl SARCOutputFile {
    pub fn export(&self, folder: &Path) -> Result<(), Box<Error>> {
        let mut file = File::create(folder.join(Path::new(&self.name)).as_path())?;
        file.write_all(&self.data)?;
        file.sync_all()?;
        Ok(())
    }
}
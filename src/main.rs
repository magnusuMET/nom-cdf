mod parser;
use anyhow::{anyhow, Result};
use parser::*;
use std::io::Read;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "parseCDF", about = "Parse CDF-1,2,5 files")]
struct Options {
    #[structopt(parse(from_os_str))]
    filename: PathBuf,
}

fn main() -> Result<()> {
    let opt = Options::from_args();
    let mut contents = vec![];
    {
        let mut file = std::fs::File::open(&opt.filename)?;
        file.read_to_end(&mut contents)?;
    }

    let header = cdf_parser(&contents)
        .map_err(|_| anyhow!("Could not parse file, is this a valid CDF-1, 2, or 5 file?"))?
        .1;
    let file = File {
        header,
        data: contents.to_vec(),
    };
    println!("Version = {:?}", file.header.version);
    println!("Number of records: {}", file.header.numrecs.unwrap_or(0));
    println!("Dimension list:");
    for (id, dim) in file
        .header
        .dim_list
        .unwrap_or_else(|| Vec::new())
        .iter()
        .enumerate()
    {
        println!("\t{}: len({}) id: {}", dim.name, dim.len, id);
    }
    println!("Attribute list:");
    for att in file.header.gatt_list.unwrap_or_else(|| Vec::new()) {
        println!("\t{} typ: {:?}", att.name, att.typ);
    }
    println!("Variable list:");
    for var in file.header.var_list.unwrap_or_else(|| Vec::new()) {
        println!("\t{} typ({:?}) dimids({:?})", var.name, var.typ, var.dimids);
        for att in var.atts.unwrap_or_else(|| Vec::new()) {
            println!("\t\t{} typ: {:?}", att.name, att.typ);
        }
    }

    Ok(())
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Version {
    CDF1,
    CDF2,
    CDF5,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Type {
    I8,
    U8,
    Char,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
}
impl Type {
    pub fn byte_size(self) -> usize {
        match self {
            Self::I8 | Self::U8 | Self::Char => 1,
            Self::I16 | Self::U16 => 2,
            Self::I32 | Self::U32 | Self::F32 => 4,
            Self::I64 | Self::U64 | Self::F64 => 8,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Dimension {
    pub name: String,
    pub len: u64,
}
#[derive(Clone, Debug)]
pub struct Attribute {
    pub name: String,
    pub typ: Type,
    pub data: Vec<u8>,
}
#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub dimids: Vec<u64>,
    pub atts: Option<Vec<Attribute>>,
    pub typ: Type,
    pub vsize: u64,
    pub begin: u64,
}
#[derive(Debug, Clone)]
pub struct FileHeader {
    pub version: Version,
    pub numrecs: Option<u64>,
    pub dim_list: Option<Vec<Dimension>>,
    pub gatt_list: Option<Vec<Attribute>>,
    pub var_list: Option<Vec<Variable>>,
}
#[derive(Debug, Clone)]
pub struct File {
    pub header: FileHeader,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Data(pub Vec<u8>);

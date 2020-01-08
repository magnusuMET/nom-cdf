mod parser;
use parser::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let contents: &[u8] = include_bytes!("../coads_climatology.nc");

    let header = cdf_parser(contents)?.1;
    let file = File {
        header,
        data: contents.to_vec(),
    };
    println!("{:?}", file.header);

    let coads = file
        .header
        .var_list
        .as_ref()
        .unwrap()
        .iter()
        .find(|&var| var.name == "COADSX")
        .unwrap();

    println!("{:?}", coads);
    let coads_ptr = coads.begin;
    let typ = coads.typ;
    assert_eq!(typ, Type::F64);

    let dims = &coads.dimids;
    let len = dims
        .iter()
        .map(|&i| file.header.dim_list.as_ref().unwrap()[i as usize].len as usize)
        .product::<usize>();

    let data: Vec<f64> = file.data[coads_ptr as usize..]
        .chunks_exact(typ.byte_size())
        .take(len)
        .map(|x| {
            let mut y = [0; 8];
            y.copy_from_slice(&x[..8]);
            f64::from_be_bytes(y)
        })
        .collect::<Vec<f64>>();
    println!("{:?}", data);

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

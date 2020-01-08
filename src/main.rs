use nom::branch::*;
use nom::bytes::complete::*;
use nom::combinator::*;
use nom::multi::*;
use nom::sequence::*;
use nom::IResult;
use nom::number::complete::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let contents: &[u8] = include_bytes!("../coads_climatology.nc");

    parser(contents)?;

    #[derive(Copy, Clone, Debug)]
    enum Version {
        CDF1,
        CDF2,
        CDF5,
    }

    fn magic(input: &[u8]) -> IResult<&[u8], Version> {
        fn version(input: &[u8]) -> IResult<&[u8], Version> {
            alt((map(tag(&[0x01]), |_| Version::CDF1),
                map(tag(&[0x02]), |_| Version::CDF2),
                map(tag(&[0x05]), |_| Version::CDF5),
                ))(input)
        }

        preceded(tag("CDF"), version)(input)
    }
    fn numrecs(input: &[u8]) -> IResult<&[u8], Option<u32>> {
        fn streaming(input: &[u8]) -> IResult<&[u8], ()> {
            map(tag(&[0xff, 0xff, 0xff, 0xff]), |_| ())(input)
        }
        alt((map(streaming, |_| None), map(non_neg, |x| Some(x))))(input)
    }

    fn non_neg(input: &[u8]) -> IResult<&[u8], u32> {
        be_u32(input)
        // nom::number::complete::be_u64(input)
    }

    #[derive(Clone, Debug)]
    struct Dimension {
        name: String,
        len: u32,
    }
    fn absent(input: &[u8]) -> IResult<&[u8], ()> {
        fn zero(input: &[u8]) -> IResult<&[u8], ()> {
            map(tag(&[0, 0, 0, 0]), |_| ())(input)
        }
        fn zero64(input: &[u8]) -> IResult<&[u8], ()> {
            map(tag(&[0, 0, 0, 0, 0, 0, 0, 0]), |_| ())(input)
        }
        // if version == Version::CDF5 {
        //     map(pair(zero, zero64), |_| ())(input)
        // } else {
        map(pair(zero, zero), |_| ())(input)
        // }
    }
    fn name<'a>(input: &'a [u8], orig: &'a [u8]) -> IResult<&'a [u8], String> {
        let (i, num) = non_neg(input)?;
        let (i, x) = map(take(num as usize), |s: &[u8]| {
            String::from(std::str::from_utf8(s).unwrap())
        })(i)?;
        let (i, _) = padding(i, orig)?;
        Ok((i, x))
    }
    fn dimlist<'a>(input: &'a [u8], orig: &'a [u8]) -> IResult<&'a [u8], Option<Vec<Dimension>>> {
        fn nc_dimension(input: &[u8]) -> IResult<&[u8], ()> {
            map(tag(&[0, 0, 0, 0x0a]), |_| ())(input)
        }
        fn dim<'a>(input: &'a [u8], orig: &'a [u8]) -> IResult<&'a [u8], Dimension> {
            let (i, name) = name(input, orig)?;
            let (i, len) = non_neg(i)?;

            Ok((i, Dimension { name, len }))
        }

        let (i, num_dims) = alt((
            map(absent, |_| None),
            map(preceded(nc_dimension, non_neg), |x| Some(x)),
        ))(input)?;

        if let Some(s) = num_dims {
            let mut v = Vec::with_capacity(s as usize);
            let mut i = i;
            for _ in 0..s {
                let id = dim(i, orig)?;
                i = id.0;
                v.push(id.1);
            }
            Ok((i, Some(v)))
        } else {
            Ok((i, None))
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum Type {
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
        fn byte_size(&self) -> usize {
            match self {
                Self::I8 | Self::U8 | Self::Char => 1,
                Self::I16 | Self::U16 => 2,
                Self::I32 | Self::U32 | Self::F32 => 4,
                Self::I64 | Self::U64 | Self::F64 => 8,
            }
        }
    }
    #[derive(Clone, Debug)]
    struct Attribute {
        name: String,
        typ: Type,
        data: Vec<u8>,
    }
    fn padding<'a>(input: &'a [u8], orig: &'a [u8]) -> IResult<&'a [u8], ()> {
        use nom::Offset;
        let grab = orig.offset(input);
        let offset: usize = match grab % 4 {
            0 => 0,
            1 => 3,
            2 => 2,
            3 => 1,
            _ => unreachable!(),
        };

        map(take(offset), |_| ())(input)
    }
    fn nc_type(input: &[u8]) -> IResult<&[u8], Type> {
        alt((
            map(tag(&[0, 0, 0, 0x01]), |_| Type::Char),
            map(tag(&[0, 0, 0, 0x02]), |_| Type::I8),
            map(tag(&[0, 0, 0, 0x03]), |_| Type::I16),
            map(tag(&[0, 0, 0, 0x04]), |_| Type::I32),
            map(tag(&[0, 0, 0, 0x05]), |_| Type::F32),
            map(tag(&[0, 0, 0, 0x06]), |_| Type::F64),
            map(tag(&[0, 0, 0, 0x07]), |_| Type::U8),
            map(tag(&[0, 0, 0, 0x08]), |_| Type::U16),
            map(tag(&[0, 0, 0, 0x09]), |_| Type::U32),
            map(tag(&[0, 0, 0, 0x0a]), |_| Type::I64),
            map(tag(&[0, 0, 0, 0x0b]), |_| Type::U64),
        ))(input)
    }
    fn att_list<'a>(input: &'a [u8], orig: &'a [u8]) -> IResult<&'a [u8], Option<Vec<Attribute>>> {
        fn nc_attribute(input: &[u8]) -> IResult<&[u8], ()> {
            map(tag(&[0, 0, 0, 0x0c]), |_| ())(input)
        }
        fn attr<'a>(input: &'a [u8], orig: &'a [u8]) -> IResult<&'a [u8], Attribute> {
            let (i, name) = name(input, orig)?;
            let (i, typ) = nc_type(i)?;
            let (i, nelems) = non_neg(i)?;
            let (i, values) = map(take(nelems as usize * typ.byte_size()), |x: &[u8]| {
                x.to_vec()
            })(i)?;
            let (i, _) = padding(i, orig)?;

            Ok((
                i,
                Attribute {
                    name,
                    typ,
                    data: values,
                },
            ))
        }
        let (i, atts) = alt((map(absent, |_| false), map(nc_attribute, |_| true)))(input)?;
        if atts {
            let (i, nelems) = non_neg(i)?;
            let mut attributes = Vec::with_capacity(nelems as usize);
            let mut i = i;
            for _ in 0..nelems {
                let id = attr(i, orig)?;
                i = id.0;
                attributes.push(id.1);
            }
            Ok((i, Some(attributes)))
        } else {
            Ok((i, None))
        }
    }
    fn gatt_list<'a>(input: &'a [u8], orig: &'a [u8]) -> IResult<&'a [u8], Option<Vec<Attribute>>> {
        att_list(input, orig)
    }

    #[derive(Debug, Clone)]
    struct Variable {
        name: String,
        dimids: Vec<u32>,
        atts: Option<Vec<Attribute>>,
        typ: Type,
        vsize: u32,
        begin: u32,
    }

    fn var_list<'a>(input: &'a [u8], orig: &'a [u8]) -> IResult<&'a [u8], Option<Vec<Variable>>> {
        fn nc_variable(input: &[u8]) -> IResult<&[u8], ()> {
            map(tag(&[0, 0, 0, 0x0b]), |_| ())(input)
        }
        fn offset(input: &[u8]) -> IResult<&[u8], u32> {
            be_u32(input)
            // if version == CDF2 | version == CDF5 {
            //     be_u64(input)
            // }
        }
        fn var<'a>(input: &'a [u8], orig: &'a [u8]) -> IResult<&'a [u8], Variable> {
            let mut i = input;
            let id = name(i, orig)?;
            i = id.0;
            let name = id.1;
            let inelems = non_neg(i)?;
            i = inelems.0;
            let nelems = inelems.1;
            let idimids = count(non_neg, nelems as usize)(i)?;
            i = idimids.0;
            let dimids = idimids.1;
            let iatts = att_list(i, orig)?;
            i = iatts.0;
            let atts = iatts.1;
            let itype = nc_type(i)?;
            i = itype.0;
            let typ = itype.1;
            let ivsize = non_neg(i)?;
            i = ivsize.0;
            let vsize = ivsize.1;
            let ibegin = offset(i)?;
            i = ibegin.0;
            let begin = ibegin.1;

            let v = Variable {
                name,
                dimids,
                typ,
                vsize,
                atts,
                begin,
            };
            Ok((i, v))
        }
        let (i, vars) = alt((map(absent, |_| false), map(nc_variable, |_| true)))(input)?;
        if vars {
            let (i, nelems) = non_neg(i)?;
            let mut variables = Vec::with_capacity(nelems as usize);
            let mut i = i;
            for _ in 0..nelems {
                let iv = var(i, orig)?;
                i = iv.0;
                variables.push(iv.1);
            }
            Ok((i, Some(variables)))
        } else {
            Ok((i, None))
        }
    }

    #[derive(Debug, Clone)]
    struct FileHeader {
        version: Version,
        numrecs: Option<u32>,
        dim_list: Option<Vec<Dimension>>,
        gatt_list: Option<Vec<Attribute>>,
        var_list: Option<Vec<Variable>>
    }

    fn header(input: &[u8]) -> IResult<&[u8], FileHeader> {
        let (i, version) = magic(input)?;
        let (i, numrecs) = numrecs(i)?;
        let (i, dim_list) = dimlist(i, input)?;
        let (i, gatt_list) = gatt_list(i, input)?;
        let (i, var_list) = var_list(i, input)?;

        let _data = i;

        Ok((i, FileHeader {
            version,
            numrecs,
            dim_list,
            gatt_list,
            var_list,
        }))
    }

    #[derive(Debug, Clone)]
    struct File {
        header: FileHeader,
    }

    fn parser(input: &[u8]) -> IResult<&[u8], File> {
        let (i, header) = header(input)?;
        println!("{:?}", header);
        Ok((i, File {
            header
        }))
    }

    Ok(())
}

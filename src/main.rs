mod parser;
use parser::cdf_parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let contents: &[u8] = include_bytes!("../coads_climatology.nc");

    let file = cdf_parser(contents)?.1;
    println!("{:?}", file.header);

    Ok(())
}

use std::fs;
use clap::Parser;

use docxtools::xml_util::XMLUtil;
use docxtools::zip_util::ZipUtil;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    name: String,

    /// Number of times to greet
    #[arg(short, long, default_value_t = 1)]
    count: u8,
}
fn main() {
    let args = Args::parse();

    for _ in 0..args.count {
        println!("Hello {}!", args.name)
    }
    
    std::process::exit(real_main());
}

fn real_main() -> i32 {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 4 {
        println!("Usage: {} <in-filename> <out-filename> <tempdir>", args[0]);
        return 1;
    }

    let src_file = &*args[1];
    let fname = std::path::Path::new(src_file);
    let file = fs::File::open(fname).unwrap();

    let dest_file = &*args[2];

    let temp_dir = &*args[3];
    let tname = std::path::Path::new(temp_dir);

    ZipUtil::read_zip(file, tname).unwrap();

    XMLUtil::read_xmls(temp_dir); // .unwrap();

    ZipUtil::write_zip(temp_dir, dest_file).unwrap();

    0
}

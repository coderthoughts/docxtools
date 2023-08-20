use clap::Parser;

use docxtools::xml_util::XMLUtil;
use docxtools::zip_util::ZipUtil;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    // /// Name of the person to greet
    // #[arg(short, long)]
    // name: String,

    // /// Number of times to greet
    // #[arg(short, long, default_value_t = 1)]
    // count: u8,
    
    #[arg(long)]
    command: String,

    #[arg(long)]
    grep_pattern: String,

    #[arg(short, long)]
    in_file: String,

    // #[arg(short, long)]
    // out_file: String,

    #[arg(short, long)]
    temp_dir: String
}
fn main() {
    let args = Args::parse();

    // for _ in 0..args.count {
    //     println!("Hello {}!", args.name)
    // }
    
    std::process::exit(real_main(args));
}

fn real_main(args: Args) -> i32 {
    // let args: Vec<_> = std::env::args().collect();
    // if args.len() < 4 {
    //     println!("Usage: {} <in-filename> <out-filename> <tempdir>", args[0]);
    //     return 1;
    // }

    // let src_file = &*args[1];
    let src_file = args.in_file;
    // let dest_file = args.out_file;
    let temp_dir = args.temp_dir;

    ZipUtil::read_zip(&src_file, &temp_dir).unwrap();

    match args.command.as_str() {
        "grep" => {
            XMLUtil::grep_xml(&temp_dir, &args.grep_pattern)
        },
        _ => panic!("Unknown command {}", args.command)
    }
    // XMLUtil::read_xmls(&temp_dir); // .unwrap();

    // ZipUtil::write_zip(&temp_dir, &dest_file).unwrap();

    0
}

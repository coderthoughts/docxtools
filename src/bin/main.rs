use clap::Parser;

use docxtools::xml_util::XMLUtil;
use docxtools::zip_util::ZipUtil;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    command: String,

    #[arg(long)]
    grep_pattern: Option<String>,

    #[arg(short, long)]
    in_file: String,

    // #[arg(short, long)]
    // out_file: String,

    // TODO automatically pick a temp dir
    #[arg(short, long)]
    temp_dir: String
}
fn main() {
    let args = Args::parse();

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
        "cat" => {
            XMLUtil::cat(&temp_dir);
        },
        "grep" => {
            XMLUtil::grep_xml(&temp_dir, &args.grep_pattern.unwrap())
        },
        _ => panic!("Unknown command {}", args.command)
    }
    // XMLUtil::read_xmls(&temp_dir); // .unwrap();

    // ZipUtil::write_zip(&temp_dir, &dest_file).unwrap();

    // TODO delete temp dir
    0
}

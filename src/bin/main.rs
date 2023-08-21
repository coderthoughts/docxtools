use clap::Parser;
use tempfile::tempdir;

use docxtools::xml_util::XMLUtil;
use docxtools::zip_util::ZipUtil;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    command: String,

    #[arg(long)]
    grep_pattern: Option<String>,

    /// The docx file to operate on.
    #[arg(short, long)]
    in_file: String,

    // #[arg(short, long)]
    // out_file: String,

    /// The temporary directory to use. If not specified a system temp directory
    /// will be used and cleaned after use.
    #[arg(short, long)]
    temp_dir: Option<String>
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

    let fstempdir = tempdir().unwrap();

    let temp_dir;
    if let Option::Some(v) = args.temp_dir {
        temp_dir = v;
    } else {
        temp_dir = fstempdir.path().to_string_lossy().to_string();
    }

    // let temp_dir = args.temp_dir;

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

    // Delete temp dir
    fstempdir.close().unwrap();

    0
}

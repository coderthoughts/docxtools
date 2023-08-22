use clap::{Args, Parser, Subcommand};
use tempfile::tempdir;

use docxtools::xml_util::XMLUtil;
use docxtools::zip_util::ZipUtil;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// The docx file to operate on.
    in_file: String,

    /// The temporary directory to use. If not specified a system temp directory
    /// will be used and cleaned after use.
    #[arg(short, long)]
    temp_dir: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List the text from the document to the console
    Cat(CatArgs),

    /// Search the text in the document like 'grep'
    Grep(GrepArgs),
}

#[derive(Args)]
struct CatArgs {
}

#[derive(Args)]
struct GrepArgs {
    /// The regular expression to search for
    regex: String,
}

fn main() {
    let args = Cli::parse();

    std::process::exit(real_main(args));
}

fn real_main(args: Cli) -> i32 {
    let src_file = args.in_file;
    // let dest_file = args.out_file;

    let fstempdir = tempdir().unwrap();

    let temp_dir;
    if let Option::Some(v) = args.temp_dir {
        temp_dir = v;
    } else {
        temp_dir = fstempdir.path().to_string_lossy().to_string();
    }

    ZipUtil::read_zip(&src_file, &temp_dir).unwrap();

    match &args.command {
        Commands::Cat(_) => {
            XMLUtil::cat(&temp_dir);
        },
        Commands::Grep(grep_args) => {
            XMLUtil::grep_xml(&temp_dir, &grep_args.regex)
        }
    }

    // XMLUtil::read_xmls(&temp_dir); // .unwrap();

    // ZipUtil::write_zip(&temp_dir, &dest_file).unwrap();

    // Delete temp dir
    fstempdir.close().unwrap();

    0
}

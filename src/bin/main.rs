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

    Replace(ReplaceArgs),
}

#[derive(Args)]
struct CatArgs {
}

#[derive(Args)]
struct GrepArgs {
    /// The regular expression to search for
    regex: String,
}

#[derive(Args)]
struct ReplaceArgs {
    regex: String,

    replace: String,

    out_file: String
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
        },
        Commands::Replace(replace_args) => {
            XMLUtil::replace_xml(&temp_dir,
                &replace_args.regex, &replace_args.replace,
                &replace_args.out_file);
        }
    }

    // Delete temp dir
    fstempdir.close().unwrap();

    0
}

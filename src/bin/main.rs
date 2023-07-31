use std::fs;

use docxtools::zip_util::ZipUtil;

fn main() {
    std::process::exit(real_main());
}

fn real_main() -> i32 {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 3 {
        println!("Usage: {} <filename> <tempdir>", args[0]);
        return 1;
    }

    let src_file = &*args[1];
    let fname = std::path::Path::new(src_file);
    let file = fs::File::open(fname).unwrap();

    let temp_dir = &*args[2];
    let tname = std::path::Path::new(temp_dir);

    ZipUtil::read_zip(file, tname).unwrap();

    let dest_file = src_file.to_owned() + ".xyz";

    ZipUtil::write_zip(temp_dir, &dest_file).unwrap();

    0
}

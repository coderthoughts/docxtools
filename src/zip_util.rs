use std::fs;
use std::io;
use std::io::{Read, Write};
use std::path::Path;
use zip::result::ZipError;
use zip::write::FileOptions;
use walkdir::{DirEntry, WalkDir};

pub struct ZipUtil {
}

impl ZipUtil {
    pub fn read_zip(
        zip_file: &str,
        dest_dir: &str
    ) -> zip::result::ZipResult<()> {
        let fname = std::path::Path::new(zip_file);
        let file = fs::File::open(fname)?;

        let tname = std::path::Path::new(dest_dir);

        Self::read_zip_file(file, tname)
    }

    fn read_zip_file(
        file: fs::File,
        temp_path: &Path
    ) -> zip::result::ZipResult<()> {
        let mut archive = zip::ZipArchive::new(file)?;

        let outpathbase = temp_path.to_owned();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpathfn = match file.enclosed_name() {
                Some(path) => path.to_owned(),
                None => continue,
            };

            let outpath = outpathbase.join(outpathfn);

            {
                let comment = file.comment();
                if !comment.is_empty() {
                    println!("File {i} comment: {comment}");
                }
            }

            if (*file.name()).ends_with('/') {
                // println!("File {} extracted to \"{}\"", i, outpath.display());
                fs::create_dir_all(&outpath)?;
            } else {
                // println!(
                //     "File {} extracted to \"{}\" ({} bytes)",
                //     i,
                //     outpath.display(),
                //     file.size()
                // );
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(p)?;
                    }
                }
                let mut outfile = fs::File::create(&outpath)?;
                io::copy(&mut file, &mut outfile)?;
            }

            // Get and Set permissions
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                if let Some(mode) = file.unix_mode() {
                    fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
                }
            }
        }

        Ok(())
    }

    pub fn write_zip(
        src_dir: &str,
        dst_file: &str,
    ) -> zip::result::ZipResult<()> {
        if !Path::new(src_dir).is_dir() {
            return Err(ZipError::FileNotFound);
        }

        let path = Path::new(dst_file);
        let file = fs::File::create(path)?;

        let walkdir = WalkDir::new(src_dir);
        let it = walkdir.into_iter();

        Self::deflate(&mut it.filter_map(|e| e.ok()), src_dir, file,
            zip::CompressionMethod::Deflated)?;

        Ok(())
    }

    fn deflate<T>(
        it: &mut dyn Iterator<Item = DirEntry>,
        prefix: &str,
        writer: T,
        method: zip::CompressionMethod,
    ) -> zip::result::ZipResult<()>
    where
        T: io::Write + io::Seek,
    {
        let mut zip = zip::ZipWriter::new(writer);
        let options = FileOptions::default()
            .compression_method(method)
            .unix_permissions(0o755);

        let mut buffer = Vec::new();
        for entry in it {
            let path = entry.path();
            let name = path.strip_prefix(Path::new(prefix)).unwrap();

            // Write file or directory explicitly
            // Some unzip tools unzip files with directory paths correctly, some do not!
            if path.is_file() {
                // println!("adding file {path:?} as {name:?} ...");
                #[allow(deprecated)]
                zip.start_file_from_path(name, options)?;
                let mut f = fs::File::open(path)?;

                f.read_to_end(&mut buffer)?;
                zip.write_all(&buffer)?;
                buffer.clear();
            } else if !name.as_os_str().is_empty() {
                // Only if not root! Avoids path spec / warning
                // and mapname conversion failed error on unzip
                // println!("adding dir {path:?} as {name:?} ...");
                #[allow(deprecated)]
                zip.add_directory_from_path(name, options)?;
            }
        }
        zip.finish()?;
        Result::Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::file_util::FileUtil;
    use super::ZipUtil;
    use std::{path::Path, fs, io};
    use walkdir::WalkDir;
    use testdir::testdir;

    #[test]
    fn test_unzip() -> io::Result<()> {
        let zipfile = "./src/test/test_zip.zip";
        let outdir = testdir!();

        ZipUtil::read_zip(zipfile, &outdir.to_string_lossy())?;

        let wd = WalkDir::new(&outdir);
        let extracts: Vec<String> = wd.into_iter()
            .map(|e| FileUtil::get_sub_path(&e.unwrap().path(), &outdir.to_string_lossy()))
            .filter(|e| !e.starts_with("/"))
            .filter(|e| e.contains('.'))
            .collect();

        assert!(extracts.contains(&"foo.test.txt".into()));
        assert!(extracts.contains(&"empty.file".into()));
        assert!(extracts.contains(&"sub/sub/[Content_Types].xml".into()));
        assert_eq!(3, extracts.len(), "Should be only 3 files");

        let empty_file = Path::new(&outdir).join("empty.file");
        assert!(empty_file.is_file());
        assert_eq!(0, empty_file.metadata()?.len(), "This file is empty");

        let test_file = Path::new(&outdir).join("foo.test.txt");
        assert!(test_file.is_file());
        assert_eq!(4, test_file.metadata()?.len());
        let test_cont = fs::read_to_string(test_file)?;
        assert_eq!("foo\n", test_cont);

        let ct_file = Path::new(&outdir).join("sub/sub/[Content_Types].xml");
        assert!(ct_file.is_file());
        assert_eq!(1312, ct_file.metadata()?.len());
        let ct_cont = fs::read_to_string(ct_file)?;
        assert!(ct_cont.starts_with("<?xml version"));

        Ok(())
    }
}
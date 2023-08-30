use regex::Regex;
use std::fs::File;
use std::io::{BufReader,Read};
use std::path::Path;
use xml_dom::parser::read_reader;
use xml_dom::level2::*;
use unicode_bom::Bom;
use walkdir::WalkDir;

use crate::file_util::FileUtil;
use crate::zip_util::ZipUtil;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Mode {
    Attribute,
    Value
}

pub struct XMLUtil {
}

impl XMLUtil {
    pub fn cat(dir: &str, src_file: &str) {
        Self::snr_xml(Mode::Value, dir, src_file, None, None, None, None);
    }

    pub fn grep_xml(dir: &str, src_file: &str, pattern: &str) {
        Self::snr_xml(Mode::Value, dir, src_file, None, Some(pattern), None, None);
    }

    pub fn replace_xml(dir: &str, src_file: &str, pattern: &str, replace: &str, output_file: &Option<String>) {
        let out_file = match output_file {
            Some(of) => of.as_str(),
            None => src_file
        };

        Self::snr_xml(Mode::Value, dir, src_file, Some(vec!("word/document.xml")), Some(pattern), Some(replace), Some(out_file));
    }

    pub fn replace_attr(dir: &str, src_file: &str, pattern: &str, replace: &str, output_file: &Option<String>) {
        let out_file = match output_file {
            Some(of) => of.as_str(),
            None => src_file
        };

        Self::snr_xml(Mode::Attribute, dir, src_file, Some(vec!("word/_rels/document.xml.rels")), Some(pattern), Some(replace), Some(out_file));
    }

    fn snr_xml(mode: Mode, dir: &str, src_file: &str, files: Option<Vec<&str>>, pattern: Option<&str>, replace: Option<&str>, output_file: Option<&str>) {
        let mut base_dir = dir.to_owned();
        if !dir.ends_with("/") {
            base_dir.push('/');
        }

        let regex;
        if let Some(regexpat) = pattern {
            regex = Some(Regex::new(regexpat).unwrap());
        } else {
            regex = None;
        }

        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let sub_path = FileUtil::get_sub_path(entry.path(), &base_dir);

                if let Some(file_list) = &files {
                    if !file_list.contains(&sub_path.as_str()) {
                        continue;
                    }
                } else {
                    if !(sub_path.ends_with(".xml")) {
                        continue;
                    }
                }

                Self::snr_xml_file(mode, entry.path(), &regex, &replace, src_file);
            }
        }

        if let Some(outfile) = output_file {
            ZipUtil::write_zip(dir, outfile).unwrap();
        }
    }

    fn snr_xml_file(mode: Mode, path: &Path, regex: &Option<Regex>, replace: &Option<&str>, src_file: &str) {
        // detect BOM (Byte Order Mark)
        let bom = Self::get_bom(path);
        let f = File::open(path).unwrap(); // TODO
        let mut r = BufReader::new(f);

        if bom != Bom::Null {
            // Remove the BOM bytes from the stream as they will cause the XML parsing to fail
            let len = bom.len();
            let mut bom_prefix = vec![0; len];
            let _ = r.read_exact(&mut bom_prefix);
        }

        let dom_res = read_reader(r);

        match dom_res {
            Ok(dom) => {
                let changed = match mode {
                    Mode::Attribute =>
                        Self::snr_xml_attribute(&dom, regex, replace, src_file),
                    Mode::Value =>
                        Self::snr_xml_node(&dom, regex, replace, src_file)
                };

                if changed {
                    std::fs::write(path, dom.to_string()).unwrap();
                }
            },
            Err(e) => println!("Problem with XML file {}: {}", path.display(), e)
        }
    }

    fn snr_xml_attribute(node: &RefNode, regex: &Option<Regex>, replace: &Option<&str>, src_file: &str)
        -> bool {
        let mut changed = false;

        for n in node.child_nodes() {
            for (_, mut attr) in n.attributes() {
                // let v = av.value();
                // println!("Name: {} = {:?}", an, v);
                if let Some(v) = attr.value() {
                    if v.len() == 0 {
                        continue;
                    }

                    match regex {
                        Some(r) => {
                            if r.is_match(&v) {
                                println!("{}: {}={}", src_file, attr.node_name(), v);
                                if let Some(repl) = replace {
                                    let res = r.replace_all(&v, *repl);
                                    let _ = attr.set_value(&res);  // TODO
                                    changed = true;
                                }
                            }
                        },
                        None => {
                            println!("{}: {}={}", src_file, attr.node_name(), v);
                        }
                    }
                }
            }
            changed |= Self::snr_xml_attribute(&n, regex, replace, src_file);
        }

        changed
    }

    fn snr_xml_node(node: &RefNode, regex: &Option<Regex>, replace: &Option<&str>, src_file: &str)
        -> bool {
        let mut changed = false;

        for mut n in node.child_nodes() {
            if let Some(v) = n.node_value() {
                if v.len() == 0 {
                    continue;
                }

                match regex {
                    Some(r) => {
                        if r.is_match(&v) {
                            println!("{}: {}", src_file, v);
                            if let Some(repl) = replace {
                                let res = r.replace_all(&v, *repl);
                                let _ = n.set_node_value(&res); // TODO
                                changed = true;
                            }
                        }
                    },
                    None => {
                        println!("{}: {}", src_file, v);
                    }
                }
            }
            changed |= Self::snr_xml_node(&n, regex, replace, src_file);
        }

        changed
    }

    fn get_bom(path: &Path) -> Bom {
        let mut file = File::open(path).unwrap();
        Bom::from(&mut file)
    }
}

#[cfg(test)]
mod tests {
    use super::XMLUtil;
    use std::{fs, io};
    use std::path::Path;
    use testdir::testdir;

    #[test]
    fn test_replace() -> io::Result<()> {
        let orgdir = "./src/test/test_tree2";
        let testdir = testdir!();

        copy_dir_all(orgdir, &testdir)?;

        let before = fs::read_to_string("./src/test/test_tree2/word/document.xml")?;
        assert!(before.contains("And some some more text"), "Precondition");
        assert!(before.contains("and then some"), "Precondition");
        assert!(before.contains("Something here"), "Precondition");
        assert!(before.contains(">some<"), "Precondition");
        assert!(before.contains(">Some <"), "Precondition");
        assert!(!before.contains("zzz"), "Precondition");

        XMLUtil::replace_xml(&testdir.to_string_lossy(), "my-source.docx",
            "[Ss]ome", "zzz", &None);

        // Check that the replacement worked as expected
        let after = fs::read_to_string(testdir.join("word/document.xml"))?;
        assert!(after.contains("And zzz zzz more text"));
        assert!(after.contains("and then zzz"));
        assert!(after.contains("zzzthing here"));
        assert!(after.contains(">zzz"));
        assert!(!after.contains("some"));
        assert!(!after.contains("Some"));

        Ok(())
    }

    #[test]
    fn test_replace_hyperlink() -> io::Result<()> {
        let orgdir = "./src/test/test_tree2";
        let testdir = testdir!();

        copy_dir_all(orgdir, &testdir)?;

        let before_doc = fs::read_to_string("./src/test/test_tree2/word/document.xml")?;
        let before = fs::read_to_string("./src/test/test_tree2/word/_rels/document.xml.rels")?;

        assert!(before.contains("Target=\"http://www.example.com/\""), "Precondition");
        assert!(before_doc.contains(">www.example.com<"), "Precondition");

        XMLUtil::replace_attr(&testdir.to_string_lossy(), "my-source.docx",
            "www.example.com", "foobar.org", &None);

        let after_doc = fs::read_to_string("./src/test/test_tree2/word/document.xml")?;
        let after = fs::read_to_string(testdir.join("word/_rels/document.xml.rels"))?;

        assert!(after.contains("Target=\"http://foobar.org/\""));
        assert!(after_doc.contains(">www.example.com<"), "Should not have changed the document text");

        Ok(())
    }

    fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
        fs::create_dir_all(&dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_dir() {
                copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
            } else {
                fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
            }
        }
        Ok(())
    }
}


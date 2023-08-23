use regex::Regex;
use std::fs::File;
use std::io::{BufReader,Read};
use std::path::Path;
use xml_dom::parser::read_reader;
use xml_dom::level2::*;
use unicode_bom::Bom;
use walkdir::WalkDir;

use crate::zip_util::ZipUtil;

pub struct XMLUtil {
}

impl XMLUtil {
    pub fn cat(dir: &str) {
        Self::snr_xml(dir, None, None, None);
    }

    pub fn grep_xml(dir: &str, pattern: &str) {
        Self::snr_xml(dir, Some(pattern), None, None);
    }

    pub fn replace_xml(dir: &str, pattern: &str, replace: &str, output_file: &str) {
        Self::snr_xml(dir, Some(pattern), Some(replace), Some(output_file));
    }

    fn snr_xml(dir: &str, pattern: Option<&str>, replace: Option<&str>, output_file: Option<&str>) {
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
            if entry.file_type().is_file() && entry.file_name().to_string_lossy().ends_with(".xml") {
                Self::snr_xml_file(entry.path(), &regex, &replace, &base_dir);
            }
        }

        if let Some(outfile) = output_file {
            ZipUtil::write_zip(dir, outfile).unwrap();
        }
    }

    fn snr_xml_file(path: &Path, regex: &Option<Regex>, replace: &Option<&str>, base_dir: &str) {
        // detect BOM
        let bom = Self::getbom(path);
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
                if Self::snr_xml_node(&dom, regex, replace, path, base_dir) {
                    std::fs::write(path, dom.to_string()).unwrap();
                }
            },
            Err(e) => println!("Problem with XML file {}: {}", path.display(), e)
        }
    }

    fn snr_xml_node(node: &RefNode, regex: &Option<Regex>, replace: &Option<&str>, path: &Path, base_dir: &str)
        -> bool {
        let mut changed = false;
        let sub_path;
        let full_path = path.to_string_lossy();
        if full_path.starts_with(base_dir) {
            sub_path = &full_path[base_dir.len()..];
        } else {
            sub_path = &full_path;
        }

        for mut n in node.child_nodes() {
            if let Option::Some(v) = n.node_value() {
                if v.len() == 0 {
                    continue;
                }

                match regex {
                    Some(r) => {
                        if r.is_match(&v) {
                            println!("{}: {}", sub_path, v);
                            if let Some(repl) = replace {
                                let res = r.replace_all(&v, *repl);
                                let _ = n.set_node_value(&res);
                                changed = true;
                            }
                        }
                    },
                    None => {
                        println!("{}: {}", sub_path, v);
                    }
                }
            }
            changed |= Self::snr_xml_node(&n, regex, replace, path, base_dir);
        }

        changed
    }

    fn getbom(path: &Path) -> Bom {
        let mut file = File::open(path).unwrap();
        Bom::from(&mut file)
    }
}

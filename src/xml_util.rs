use regex::Regex;
use std::fs::File;
use std::io::{BufReader,Read};
use std::path::Path;
use xml_dom::parser::{read_xml, read_reader};
use xml_dom::level2::*;
use unicode_bom::Bom;
use walkdir::WalkDir;

use crate::zip_util::ZipUtil;

pub struct XMLUtil {
}

impl XMLUtil {
    pub fn cat(dir: &str) {
        Self::grep_xml(dir, "");
    }

    pub fn grep_xml(dir: &str, pattern: &str) {
        let mut base_dir = dir.to_owned();
        if !dir.ends_with("/") {
            base_dir.push('/');
        }

        let regex;
        if pattern.len() > 0 {
            regex = Some(Regex::new(pattern).unwrap());
        } else {
            regex = None;
        }

        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() && entry.file_name().to_string_lossy().ends_with(".xml") {
                Self::grep_xml_file(entry.path(), &regex, &base_dir);
            }
        }
    }

    fn grep_xml_file(path: &Path, regex: &Option<Regex>, base_dir: &str) {
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
            Ok(dom) => Self::grep_xml_node(&dom, regex, path, base_dir),
            Err(e) => println!("Problem with XML file {}: {}", path.display(), e)
        }
    }

    fn grep_xml_node(node: &RefNode, regex: &Option<Regex>, path: &Path, base_dir: &str) {
        let sub_path;
        let full_path = path.to_string_lossy();
        if full_path.starts_with(base_dir) {
            sub_path = &full_path[base_dir.len()..];
        } else {
            sub_path = &full_path;
        }

        for n in node.child_nodes() {
            if let Option::Some(v) = n.node_value() {
                if v.len() == 0 {
                    continue;
                }

                match regex {
                    Some(r) => {
                        if r.is_match(&v) {
                            println!("{}: {}", sub_path, v);
                        }
                    },
                    None => {
                        println!("{}: {}", sub_path, v);
                    }
                }
            }
            Self::grep_xml_node(&n, regex, path, base_dir);
        }
    }

    // replace, maybe we can unify
    pub fn replace_xml(dir: &str, pattern: &str, replace: &str, output_file: &str) {
        let mut base_dir = dir.to_owned();
        if !dir.ends_with("/") {
            base_dir.push('/');
        }

        let regex;
        if pattern.len() > 0 {
            regex = Some(Regex::new(pattern).unwrap());
        } else {
            regex = None;
        }

        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() && entry.file_name().to_string_lossy().ends_with(".xml") {
                Self::replace_xml_file(entry.path(), &regex, replace, &base_dir);
            }
        }

        ZipUtil::write_zip(dir, output_file).unwrap();
    }

    fn replace_xml_file(path: &Path, regex: &Option<Regex>, replace: &str, base_dir: &str) {
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
                if Self::replace_xml_node(&dom, regex, replace, path, base_dir) {
                    std::fs::write(path, dom.to_string()).unwrap();
                }
            },
            Err(e) => println!("Problem with XML file {}: {}", path.display(), e)
        }
    }

    fn replace_xml_node(node: &RefNode, regex: &Option<Regex>, replace: &str, path: &Path, base_dir: &str)
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
                            let res = r.replace_all(&v, replace);
                            let _ = n.set_node_value(&res);
                            changed = true;
                        }
                    },
                    None => {
                        println!("{}: {}", sub_path, v);
                    }
                }
            }
            changed |= Self::replace_xml_node(&n, regex, replace, path, base_dir);
        }

        changed
    }

    // Below here is all experimental code
    fn read_node(node: &mut RefNode) {
        for mut r in node.child_nodes() {
            let val = r.node_value();
            println!("Found child {:?}", val);

            Self::read_node(&mut r);

            let _ = match val {
                Some(s) => {
                    println!("Val: {}", s);
                    if s.eq_ignore_ascii_case("ya") {
                        let _ = r.set_node_value("RRR");
                        println!("Changed: {}", s);
                    }
                    if s.eq_ignore_ascii_case("hihih") {
                        let cl = r.child_nodes();
                        println!("Children {:?}", cl);
                        let _ = r.set_node_value("bleh");
                        println!("Changed too: {}", s);
                    }
                },
                None => {}
            };
        }
    }

    pub fn read_xmls(_root_dir: &str) {

        let mut dom = read_xml(r#"<?xml version="1.0"?>
        <hi>hihih<yo>ya</yo></hi>
        "#).unwrap();

        Self::read_node(&mut dom);
        println!("---");
        Self::read_node(&mut dom);

        println!("Resulting XML: {}", dom.to_string());
    }

    fn getbom(path: &Path) -> Bom {
        let mut file = File::open(path).unwrap();
        Bom::from(&mut file)
    }
}

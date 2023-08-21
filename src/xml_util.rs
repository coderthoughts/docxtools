use regex::Regex;
use std::fs::File;
use std::io::{BufReader,Read};
use std::path::Path;
use xml_dom::parser::{read_xml, read_reader};
use xml_dom::level2::*;
use unicode_bom::Bom;
use walkdir::WalkDir;

pub struct XMLUtil {
}

impl XMLUtil {
    pub fn cat(dir: &str) {
        Self::grep_xml(dir, "");
    }

    pub fn grep_xml(dir: &str, pattern: &str) {
        let regex;
        if pattern.len() > 0 {
            regex = Some(Regex::new(pattern).unwrap());
        } else {
            regex = None;
        }

        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() && entry.file_name().to_string_lossy().ends_with(".xml") {
                Self::grep_xml_file(entry.path(), &regex);
            }
        }
    }

    fn grep_xml_file(path: &Path, regex: &Option<Regex>) {
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
            Ok(dom) => Self::grep_xml_node(&dom, regex, path),
            Err(e) => println!("Problem with XML file {}: {}", path.display(), e)
        }
    }

    fn grep_xml_node(node: &RefNode, regex: &Option<Regex>, path: &Path) {
        for n in node.child_nodes() {
            if let Option::Some(v) = n.node_value() {
                if v.len() == 0 {
                    continue;
                }

                match regex {
                    Some(r) => {
                        if r.is_match(&v) {
                            println!("{}: {}", path.to_string_lossy(), v);
                        }
                    },
                    None => {
                        println!("{}: {}", path.to_string_lossy(), v);
                    }
                }
            }
            Self::grep_xml_node(&n, regex, path);
        }
    }

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

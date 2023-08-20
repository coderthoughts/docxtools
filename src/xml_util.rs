use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use xml_dom::parser::{read_xml, read_reader};
use xml_dom::level2::*;
use walkdir::WalkDir;
// use xml_dom::shared::rc_cell::RcRefCell;
// use xml_dom::level2::NodeImpl;
// use xml_dom::level2::NodeImpl;
// use xml_dom::level2::RcRefCell;

pub struct XMLUtil {
}

impl XMLUtil {
    pub fn grep_xml(dir: &str, pattern: &str) {

        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            // println!("{}", entry.path().display());
            if entry.file_type().is_file() {
                Self::grep_xml_file(entry.path(), pattern);
            }
        }
        // let r = "";
        // let f = File::open(dir)
        // read_reader(r);
    }

    fn grep_xml_file(path: &Path, pattern: &str) {
        let f = File::open(path).unwrap(); // TODO
        let r = BufReader::new(f);
        let dom = read_reader(r).unwrap();

        Self::grep_xml_node(&dom, pattern);

    }

    fn grep_xml_node(node: &RefNode, pattern: &str) {

        for n in node.child_nodes() {
            if let Option::Some(v) = n.node_value() {
                println!("{:?}", v);
            }
            Self::grep_xml_node(&n, pattern);
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
}

use xml_dom::parser::read_xml;
use xml_dom::level2::*;
// use xml_dom::shared::rc_cell::RcRefCell;
// use xml_dom::level2::NodeImpl;
// use xml_dom::level2::NodeImpl;
// use xml_dom::level2::RcRefCell;

pub struct XMLUtil {
}

impl XMLUtil {
    pub fn grep_xml(dir: &str, pattern: &str) {

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

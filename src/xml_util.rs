use xml_dom::parser::read_xml;
use xml_dom::level2::*;
// use xml_dom::shared::rc_cell::RcRefCell;
// use xml_dom::level2::NodeImpl;
// use xml_dom::level2::NodeImpl;
// use xml_dom::level2::RcRefCell;

pub struct XMLUtil {
}

impl XMLUtil {
    fn read_node(node: &mut RefNode) {
        for mut r in node.child_nodes() {
            let val = r.value();
            println!("Found child {:?}", val);

            Self::read_node(&mut r);

            let _ = match val {
                Some(s) => { 
                    println!("Val: {}", s);
                    if s.eq_ignore_ascii_case("ya") {
                        let _ = r.set_value("RRR"); 
                        println!("Changed: {}", s);     
                    }
                    if s.eq_ignore_ascii_case("hihih") {
                        let _ = r.set_value("bleh");
                        println!("Changed too: {}", s);
                    }
                } ,
                None => (),
            };

        }
    }

    pub fn read_xmls(_root_dir: &str) {

        let mut dom = read_xml(r#"<?xml version="1.0"?>
        <hi>hihih<yo>ya</yo></hi>
        "#).unwrap();
        // dom.
        // println!("Read dom: {:?}", dom);

        Self::read_node(&mut dom);
        Self::read_node(&mut dom);

        println!("Resulting XML: {}", dom.to_string());
        // for r in dom.child_nodes() {
        //     println!("Found child {:?}", r);
        //     // r.child_nodes()
        // }
    }
}

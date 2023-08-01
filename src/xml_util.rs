use xml_dom::parser::read_xml;

pub struct XMLUtil {
}

impl XMLUtil {
    pub fn read_xmls(root_dir: &str) {

        let dom = read_xml(r#"<?xml version="1.0"?>
        <hi>hihih</hi>
        "#);
        // dom.
        println!("Read dom: {:?}", dom);
    }
}
use quick_xml::events::{BytesEnd, Event, BytesStart};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use quick_xml::name::{LocalName, QName};
use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::Path;
use std::str;
use xml_dom::level2::{Attribute, Node, RefNode, Element};
use xml_dom::parser::read_reader;
use unicode_bom::Bom;
use walkdir::WalkDir;

use crate::file_util::FileUtil;
use crate::zip_util::ZipUtil;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Mode {
    AttrCondition {
        tagname: String,
        attrname: String,
        condkey: String,
        condval: String,
    },
    Attribute,
    Value
}

pub struct XMLUtil {
}

impl XMLUtil {
    pub fn cat(dir: &str, src_file: &str) {
        Self::snr_xml(Mode::Value, dir, src_file, None, None, None, None);
    }

    pub fn cat_rel_attr(el_name: &str, attr_name: &str, cond_key: &str, cond_val: &str,
            dir: &str, src_file: &str) {
        let fref = Self::get_rel_files(dir);

        let mode = Mode::AttrCondition {
                tagname: el_name.into(), attrname: attr_name.into(),
                condkey: cond_key.into(), condval: cond_val.into()
            };
        Self::snr_xml(mode, dir, src_file, Some(fref.iter().map(AsRef::as_ref).collect()),
            None, None, None);
    }

    pub fn grep_xml(dir: &str, src_file: &str, pattern: &str) {
        Self::snr_xml(Mode::Value, dir, src_file, None, Some(pattern), None, None);
    }

    pub fn replace_xml(dir: &str, src_file: &str, pattern: &str, replace: &str, output_file: &Option<&str>) {
        let (_, files) = Self::get_files_with_content_type(dir,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml");

        let out_file = match output_file {
            Some(of) => of,
            None => src_file
        };

        let fref = files.iter().map(AsRef::as_ref).collect();
        Self::snr_xml(Mode::Value, dir, src_file, Some(fref), Some(pattern), Some(replace), Some(out_file));
    }

    pub fn replace_rel_attr(dir: &str, src_file: &str, pattern: &str, replace: &str, output_file: &Option<&str>) {
        let fref = Self::get_rel_files(dir);

        let out_file = match output_file {
            Some(of) => of,
            None => src_file
        };

        Self::snr_xml(Mode::Attribute, dir, src_file, Some(fref.iter().map(AsRef::as_ref).collect()),
            Some(pattern), Some(replace), Some(out_file));
    }

    fn get_rel_files(dir: &str) -> Vec<String> {
        let (defaults, files) = Self::get_files_with_content_type(dir,
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml");
        let rels_extension = &defaults["application/vnd.openxmlformats-package.relationships+xml"];

        let mut rels_files = vec!();
        for f in files {
            let last_slash = f.rfind('/').expect(&f);
            let mut new_fn = String::new();
            new_fn.push_str(&f[..last_slash]);
            new_fn.push_str("/_");
            new_fn.push_str(rels_extension);
            new_fn.push_str(&f[last_slash..]);
            new_fn.push('.');
            new_fn.push_str(rels_extension);
            rels_files.push(new_fn);
        }

        rels_files
    }

    fn snr_xml(mode: Mode, dir: &str, src_file: &str, files: Option<Vec<&str>>, pattern: Option<&str>, replace: Option<&str>, output_file: Option<&str>) {
        let mut base_dir = dir.to_owned();
        if !dir.ends_with("/") {
            base_dir.push('/');
        }

        let regex;
        if let Some(regexpat) = pattern {
            regex = Some(Regex::new(regexpat).expect(regexpat));
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

                Self::snr_xml_file(&mode, entry.path(), &regex, &replace, src_file);
            }
        }

        if let Some(outfile) = output_file {
            ZipUtil::write_zip(dir, outfile).expect(outfile);
        }
    }

    fn snr_xml_file(mode: &Mode, path: &Path, regex: &Option<Regex>, replace: &Option<&str>, src_file: &str) {
        let mut reader = Reader::from_file(path).unwrap(); // TODO

        match mode {
            Mode::Value => Self::snr_xml_node(reader, regex, replace, src_file),
            Mode::Attribute => Self::snr_change_attribute(reader, regex, replace, src_file),
            Mode::AttrCondition { .. } => Self::snr_xml_attribute(mode, reader, &None, &None, src_file),
            
            _ => () // TODO
        }

    }

    fn snr_xml_node(mut reader: Reader<BufReader<File>>, regex: &Option<Regex>, replace: &Option<&str>, src_file: &str) {
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Err(e) => panic!("Error reading {} at position {}: {:?}", src_file, reader.buffer_position(), e),
                Ok(Event::Eof) => break,
                Ok(Event::Text(t)) => {
                    let val = t.unescape().unwrap();
                    let val_trimmed = val.trim();
                    if val_trimmed.len() > 0 {
                        println!("{}: {}", src_file, val_trimmed);
                    }
                }
                _ => (),
            }

            buf.clear();
        }
    }

    fn snr_change_attribute(mut reader: Reader<BufReader<File>>, regex: &Option<Regex>, replace: &Option<&str>, src_file: &str) {
        let mut buf = Vec::new();

        let mut writer = Writer::new(Cursor::new(Vec::new()));
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(e)) => {
                    let update_attributes = Self::update_attributes(e, regex, replace);
                    writer.write_event(Event::Empty(update_attributes)).unwrap();
                },
                Ok(Event::Start(e)) => {
                    let update_attributes = Self::update_attributes(e, regex, replace);
                    writer.write_event(Event::Start(update_attributes)).unwrap();
                },
                Ok(Event::End(e)) => {
                    writer.write_event(Event::End(e)).unwrap();
                },
                Ok(Event::Eof) => break,
                Ok(e) => writer.write_event(e).unwrap(),
                Err(e) => panic!("Error {:?}", e),
            }
        }

        println!("QQQ Writer: {}", str::from_utf8(&writer.into_inner().into_inner()).unwrap());
    }

    fn update_attributes(bs: BytesStart, regex: &Option<Regex>, replace: &Option<&str>) -> BytesStart {
        let mut es = BytesStart::clone(&bs).clear_attributes().clone();

        for attr in bs.attributes() {
            let val = quick_xml::events::attributes::Attribute::decode_and_unescape_value(&self, reader)
            let xa = quick_xml::events::attributes::Attr::DoubleQ("xx".as_bytes(), "yy".as_bytes());
            let x = quick_xml::events::attributes::Attribute::from(xa);

            es.push_attribute(x);
        }

        return es;
    }

    fn snr_xml_attribute(mode: &Mode, mut reader: Reader<BufReader<File>>, regex: &Option<Regex>, replace: &Option<&str>, src_file: &str) {
        let mut buf = Vec::new();

        // let mut writer = Writer::new(Cursor::new(Vec::new()));

        loop {
            match reader.read_event_into(&mut buf) {
                Err(e) => panic!("Error reading {} at position {}: {:?}", src_file, reader.buffer_position(), e),
                Ok(Event::Eof) => break,
                Ok(Event::Empty(e)) |
                Ok(Event::Start(e)) => {
                    match mode {
                        Mode::AttrCondition { tagname, attrname, condkey, condval } => {
                            let s = e.name();
                            let tq = QName(tagname.as_bytes());
                            if s.local_name() == tq.local_name() {
                                let attrs = e.attributes();

                                let use_node = attrs.filter(|a| a.is_ok())
                                    .map(|a| a.unwrap())
                                    .filter(|a| a.key.local_name() == QName(condkey.as_bytes()).local_name())
                                    .filter(|a| a.value == condval.as_bytes())
                                    .count();
                                if use_node > 0 {
                                    let attr = e.try_get_attribute(attrname);
                                    if let Ok(ao) = attr {
                                        if let Some(av) = ao {
                                            println!("{}: {}", src_file, str::from_utf8(&av.value).unwrap_or_default());
                                        }     
                                    }
                                }
                            }
                        },
                        /*
                        Mode::Attribute => {
                            let attrs = e.attributes();
                            for attr in attrs {
                                if let Ok(a) = attr {
                                    println!("XXXX{}: {}={}", src_file, 
                                        str::from_utf8(&a.key.local_name().as_ref()).unwrap_or_default(), 
                                        str::from_utf8(&a.value).unwrap_or_default());
                                }
                            }
                            // e.local_name()
                            // println!("{}: {}={}", src_file, attr.node_name(), v);
                        },
                         */
                        _ => ()
                    }
                    // writer.write_event(Event::Start(e)).unwrap();
                    // BytesEnd::
                    // writer.write_event(Event::End(BytesEnd::from(e))).unwrap();
                },
                // Ok(e) => writer.write_event(e).unwrap(),
                _ => (),
            }

            // buf.clear();
        }
    }

    fn _snr_xml_file(mode: &Mode, path: &Path, regex: &Option<Regex>, replace: &Option<&str>, src_file: &str) {
        // detect BOM (Byte Order Mark)
        let bom = Self::get_bom(path);
        let f = File::open(path).expect(&path.to_string_lossy());
        let mut r = BufReader::new(f);

        if bom != Bom::Null {
            // Remove the BOM bytes from the stream as they will cause the XML parsing to fail
            let len = bom.len();
            let mut bom_prefix = vec![0; len];
            r.read_exact(&mut bom_prefix).expect(&path.to_string_lossy());
        }

        let dom_res = read_reader(r);

        match dom_res {
            Ok(dom) => {
                let changed = match mode {
                    Mode::Attribute =>
                        Self::_snr_xml_attribute(&mode, &dom, regex, replace, src_file),
                    Mode::Value =>
                        Self::_snr_xml_node(&dom, regex, replace, src_file),
                    Mode::AttrCondition{ .. } =>
                        Self::_snr_xml_attribute(&mode, &dom, &None, &None, src_file)
                };

                if changed {
                    std::fs::write(path, dom.to_string()).expect(&path.to_string_lossy());
                }
            },
            Err(e) => println!("Problem with XML file {}: {}", path.display(), e)
        }
    }

    fn _snr_xml_attribute(mode: &Mode, node: &RefNode, regex: &Option<Regex>, replace: &Option<&str>, src_file: &str)
        -> bool {
        let mut changed = false;

        for n in node.child_nodes() {
            for (_, mut attr) in n.attributes() {
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
                                    attr.set_value(&res).expect(src_file);
                                    changed = true;
                                }
                            }
                        },
                        None => {
                            match mode {
                                Mode::Attribute => {
                                    println!("{}: {}={}", src_file, attr.node_name(), v);
                                },
                                Mode::AttrCondition {
                                    tagname, attrname, condkey, condval
                                } => {
                                    if &n.node_name().to_string() == tagname
                                        && &attr.node_name().to_string() == attrname {
                                        if let Some(condattr) = n.get_attribute(&condkey) {
                                            if &condattr == condval {
                                                println!("{}: {}", src_file, v);
                                            }
                                        }
                                    }
                                },
                                _ => {}
                            }
                        }
                    }
                }
            }
            changed |= Self::_snr_xml_attribute(&mode, &n, regex, replace, src_file);
        }

        changed
    }

    fn _snr_xml_node(node: &RefNode, regex: &Option<Regex>, replace: &Option<&str>, src_file: &str)
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
                                n.set_node_value(&res).expect(src_file);
                                changed = true;
                            }
                        }
                    },
                    None => {
                        println!("{}: {}", src_file, v);
                    }
                }
            }
            changed |= Self::_snr_xml_node(&n, regex, replace, src_file);
        }

        changed
    }

    fn get_bom(path: &Path) -> Bom {
        let mut file = File::open(path).expect(&path.to_string_lossy());
        Bom::from(&mut file)
    }

    fn get_content_types(dir: &str) -> (HashMap<String, String>, HashMap<String, String>) {
        let mut defaults = HashMap::new();
        let mut mappings = HashMap::new();

        let path = Path::new(dir).join("[Content_Types].xml");

        let bom = Self::get_bom(&path);
        let f = File::open(&path).expect(&path.to_string_lossy());
        let mut r = BufReader::new(f);

        if bom != Bom::Null {
            // Remove the BOM bytes from the stream as they will cause the XML parsing to fail
            let len = bom.len();
            let mut bom_prefix = vec![0; len];
            r.read_exact(&mut bom_prefix).expect(&path.to_string_lossy());
        }

        let dom_res = read_reader(r).expect(&path.to_string_lossy());
        for n in dom_res.child_nodes() {
            if n.local_name() == "Types" {
                for m in n.child_nodes() {
                    match m.local_name().as_str() {
                        "Default" => {
                            let en = m.get_attribute("Extension");
                            let ct = m.get_attribute("ContentType");

                            if en.is_some() && ct.is_some() {
                                defaults.insert(ct.expect(&path.to_string_lossy()),
                                    en.expect(&path.to_string_lossy()));
                            }
                        },
                        "Override" => {
                            let pn = m.get_attribute("PartName");
                            let ct = m.get_attribute("ContentType");

                            if pn.is_some() && ct.is_some() {
                                let pns = pn.expect(&path.to_string_lossy());
                                let rel_pn;
                                if pns.starts_with('/') {
                                    rel_pn = &pns[1..];
                                } else {
                                    rel_pn = &pns;
                                }

                                mappings.insert(rel_pn.to_owned(), ct.expect(&path.to_string_lossy()));
                            }
                        },
                        _ => {}
                    }
                }
            }
        }

        (defaults, mappings)
    }

    fn get_files_with_content_type(dir: &str, content_type: &str) -> (HashMap<String, String>, Vec<String>) {
        let (defaults, mappings) = Self::get_content_types(dir);

        let mut result = vec!();
        for (file, ct) in &mappings {
            if ct == content_type {
                result.push(file.to_owned());
            }
        }
        (defaults, result)
    }
}

#[cfg(test)]
mod tests {
    use super::XMLUtil;
    use serial_test::serial;
    use std::{fs, io};
    use std::path::Path;
    use testdir::testdir;

    // Macro to wrap around any statement to capture stdout.
    // Note tests using this need to be annotated with #[serial] as multiple concurrent
    // redirections of stdout fail.
    macro_rules! capture_stdout {
        ($test:expr) => {{
            use gag::BufferRedirect;
            use std::io::Read;

            let mut buf = BufferRedirect::stdout().unwrap();

            $test;

            let mut output = String::new();
            buf.read_to_string(&mut output).unwrap();
            drop(buf);

            output
        }};
    }

    #[test]
    #[serial] // This test has to run serially to avoid multiple tests to capture stdout
    fn test_cat() {
        let out = capture_stdout!(XMLUtil::cat("./src/test/test_tree2", "my-file.docx"));
        assert!(out.contains("my-file.docx: Testing 123"));
        assert!(out.contains("my-file.docx: Here’s a hyperlink:"));
    }

    /*
    #[test]
    #[serial] // This test has to run serially to avoid multiple tests to capture stdout
    fn test_grep() {
        let out = capture_stdout!(XMLUtil::grep_xml("./src/test/test_tree2", "doc123.docx", "[oe]re"));
        assert!(out.contains("doc123.docx: And some some more text"));
        assert!(out.contains("doc123.docx: Something here"));
        assert!(out.contains("doc123.docx: Here’s a hyperlink:"));
        assert!(out.contains("doc123.docx: And here’s just some text:"));
        assert!(!out.contains("Target"));
    }
    */

    #[test]
    #[serial]
    fn test_links() {
        let out = capture_stdout!(
            XMLUtil::cat_rel_attr (
                "Relationship", "Target",
                "Type", "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink",
                "./src/test/test_tree4", "testing789.docx"));
        assert!(out.contains("testing789.docx: http://www.example.com/somewhere"));
        assert!(out.contains("testing789.docx: https://www.example.com/somewhere"));
        assert!(out.contains("testing789.docx: file://www.example.com/infosheet.pdf"));
        assert!(!out.contains("Target=webSettings.xml"))
    }

    /*
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
            "[Ss]ome", "zzz",
            &Some(&testdir.join("output.docx").to_string_lossy()));

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
    */

    #[test]
    fn test_replace_hyperlink() -> io::Result<()> {
        let orgdir = "./src/test/test_tree2";
        let testdir = testdir!();

        copy_dir_all(orgdir, &testdir)?;

        let before_doc = fs::read_to_string("./src/test/test_tree2/word/document.xml")?;
        let before = fs::read_to_string("./src/test/test_tree2/word/_rels/document.xml.rels")?;

        assert!(before.contains("Target=\"http://www.example.com/\""), "Precondition");
        assert!(before_doc.contains(">www.example.com<"), "Precondition");

        XMLUtil::replace_rel_attr(&testdir.to_string_lossy(), "my-source.docx",
            "www.example.com", "foobar.org",
            &Some(&testdir.join("output-2.docx").to_string_lossy()));

        let after_doc = fs::read_to_string(testdir.join("word/document.xml"))?;
        let after = fs::read_to_string(testdir.join("word/_rels/document.xml.rels"))?;

        assert!(after.contains("Target=\"http://foobar.org/\""));
        assert!(after_doc.contains(">www.example.com<"), "Should not have changed the document text");

        Ok(())
    }

    /*
    #[test]
    fn test_replace_both() -> io::Result<()> {
        let orgdir = "./src/test/test_tree3";
        let testroot = testdir!();
        let testdir = testroot.join("subdir");

        copy_dir_all(orgdir, &testdir)?;

        let before = fs::read_to_string("./src/test/test_tree3/word/document2.xml")?;
        assert!(before.contains("And some some more text"), "Precondition");
        assert!(before.contains("and then some"), "Precondition");
        assert!(before.contains("Something here"), "Precondition");
        assert!(before.contains(">some<"), "Precondition");
        assert!(before.contains(">Some <"), "Precondition");
        assert!(before.contains(">www.example.com<"), "Precondition");
        assert!(!before.contains("zzz"), "Precondition");

        let before_rels = fs::read_to_string("./src/test/test_tree3/word/_rels/document2.xml.rels")?;
        assert!(before_rels.contains("Target=\"http://www.example.com/\""), "Precondition");

        XMLUtil::replace_xml(&testdir.to_string_lossy(), "my-source.docx",
            "[Ss]ome", "zzz",
            &Some(&testroot.join("output.docx").to_string_lossy()));
        XMLUtil::replace_rel_attr(&testdir.to_string_lossy(), "my-source.docx",
            "www.example.com", "foobar.org",
            &Some(&testroot.join("output-2.docx").to_string_lossy()));

        // Check that the replacement worked as expected
        let after = fs::read_to_string(testdir.join("word/document2.xml"))?;
        assert!(after.contains("And zzz zzz more text"));
        assert!(after.contains("and then zzz"));
        assert!(after.contains("zzzthing here"));
        assert!(after.contains(">zzz"));
        assert!(after.contains(">www.example.com<"), "Should not have changed the document text");
        assert!(!after.contains("some"));
        assert!(!after.contains("Some"));

        let after_rels = fs::read_to_string(testdir.join("word/_rels/document2.xml.rels"))?;
        assert!(after_rels.contains("Target=\"http://foobar.org/\""));

        Ok(())
    }
     */

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


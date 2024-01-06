use quick_xml::events::{Event, BytesStart};
use quick_xml::events::attributes::{Attr, Attribute};
use quick_xml::name::QName;
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use regex::Regex;
use std::collections::HashMap;
use std::fs::{File, self};
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::str;
use uuid::Uuid;
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

    pub fn grep_xml(_dir: &str, _src_file: &str, _pattern: &str) {
        panic!("The 'grep' functionality is currently disabled until issue #2 is fixed");
    }

    pub fn replace_xml(_dir: &str, _src_file: &str, _pattern: &str, _replace: &str, _output_file: &Option<&str>) {
        panic!("The 'replace' functionality is currently disabled until issue #2 is fixed");
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

        for entry in WalkDir::new(dir).into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file()) {
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

        if let Some(outfile) = output_file {
            ZipUtil::write_zip(dir, outfile).expect(outfile);
        }
    }

    fn snr_xml_file(mode: &Mode, path: &Path, regex: &Option<Regex>, replace: &Option<&str>, src_file: &str) {
        let reader = Reader::from_file(path).expect(&path.to_string_lossy());

        match mode {
            Mode::Value => Self::snr_xml_node(reader, src_file),
            Mode::Attribute => Self::snr_change_attribute(reader, regex, replace, src_file, path),
            Mode::AttrCondition { .. } => Self::snr_xml_attribute(mode, reader, src_file),
        }
    }

    fn snr_xml_node(mut reader: Reader<BufReader<File>>, src_file: &str) {
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Err(e) => panic!("Error reading {} at position {}: {:?}", src_file, reader.buffer_position(), e),
                Ok(Event::Eof) => break,
                Ok(Event::Text(t)) => {
                    let val = t.unescape().expect(src_file);
                    let val_trimmed = val.trim();
                    if val_trimmed.len() > 0 {
                        println!("{}: {}", src_file, val_trimmed);
                    }
                }
                _ => (),
            }

            // buf.clear(); why is this suggested in the docs?
        }
    }

    fn snr_change_attribute(mut reader: Reader<BufReader<File>>, regex: &Option<Regex>, replace: &Option<&str>, src_file: &str, output_path: &Path) {
        if regex.is_none() || replace.is_none() {
            return;
        }

        let rex = regex.as_ref().unwrap();
        let repl = replace.unwrap();

        let mut temp_res = output_path.parent().unwrap().to_owned();
        temp_res.push(format!("{}.xml", Uuid::new_v4()));

        let mut has_changes = false;
        let mut buf = Vec::new();

        let tf = File::create(&temp_res).expect(&temp_res.to_string_lossy());
        let mut writer = Writer::new(BufWriter::new(tf));
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(e)) => {
                    let (update_attributes, c) = Self::update_attributes(e, &rex, repl, src_file);
                    has_changes |= c;
                    writer.write_event(Event::Empty(update_attributes)).unwrap();
                },
                Ok(Event::Start(e)) => {
                    let (update_attributes, c) = Self::update_attributes(e, &rex, repl, src_file);
                    has_changes |= c;
                    writer.write_event(Event::Start(update_attributes)).unwrap();
                },
                Ok(Event::Eof) => break,
                Ok(e) => writer.write_event(e).unwrap(),
                Err(e) => panic!("Error {:?}", e),
            }
        }

        // This writes out the file
        writer.into_inner().into_inner().unwrap();

        if has_changes {
            // Replace the original file with the new one.
            fs::remove_file(output_path).unwrap();
            fs::rename(temp_res, output_path).unwrap();
        } else {
            // No changes, so just remove the generated file.
            fs::remove_file(temp_res).unwrap();
        }
    }

    fn update_attributes<'a>(bs: BytesStart<'a>, regex: &Regex, replace: &str, src_file: &str) -> (BytesStart<'a>, bool) {
        let mut es = BytesStart::clone(&bs);
        es.clear_attributes();

        let mut changed = false;
        for attr in bs.attributes() {
            if let Ok(a) = attr {
                let val = str::from_utf8(&a.value);

                if let Ok(v) = val {
                    let mut rval = v;
                    let rv;
                    if regex.is_match(&v) {
                        let k = a.key.local_name();
                        println!("{}: {}={}", src_file, str::from_utf8(k.as_ref()).unwrap(), v);
                        changed = true;

                        rv = regex.replace_all(&v, replace);
                        rval = &rv;
                    }
                    let na = Attr::DoubleQ(a.key.as_ref(), rval.as_bytes());
                    let new_attr = Attribute::from(na);
                    es.push_attribute(new_attr);
                }
            }
        }

        if changed {
            return (es, changed);
        } else {
            return (bs, changed);
        }
    }

    fn snr_xml_attribute(mode: &Mode, mut reader: Reader<BufReader<File>>, src_file: &str) {
        let mut buf = Vec::new();

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
                        _ => ()
                    }
                },
                _ => (),
            }
        }
    }

    fn get_content_types(dir: &str)  -> (HashMap<String, String>, HashMap<String, String>) {
        let mut defaults = HashMap::new();
        let mut mappings = HashMap::new();

        let ct_file = Path::new(dir).join("[Content_Types].xml");
        let mut reader = Reader::from_file(&ct_file).unwrap();

        let mut buf = Vec::new();
        let mut in_types = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Err(e) => panic!("Error reading {:?} at position {}: {:?}", ct_file, reader.buffer_position(), e),
                Ok(Event::Eof) => break,
                Ok(Event::Empty(e)) |
                Ok(Event::Start(e)) => {
                    if e.local_name().as_ref() == b"Types" {
                        in_types = true;
                        continue;
                    }
                    if in_types {
                        match e.local_name().as_ref() {
                            b"Default" => {
                                let en = e.try_get_attribute(b"Extension");
                                let ct = e.try_get_attribute(b"ContentType");

                                if let (Ok(e), Ok(c)) = (en, ct) {
                                    if let (Some(ev), Some(cv)) = (e, c) {
                                        defaults.insert(str::from_utf8(cv.value.as_ref()).unwrap().to_string(),
                                            str::from_utf8(ev.value.as_ref()).unwrap().to_string());
                                    }
                                }
                            },
                            b"Override" => {
                                let pn = e.try_get_attribute(b"PartName");
                                let ct = e.try_get_attribute(b"ContentType");

                                if let (Ok(p), Ok(c)) = (pn, ct) {
                                    if let (Some(pv), Some(cv)) = (p, c) {
                                        let pn = str::from_utf8(pv.value.as_ref()).unwrap();
                                        let rel_pn;
                                        if pn.starts_with('/') {
                                            rel_pn = &pn[1..];
                                        } else {
                                            rel_pn = pn;
                                        }
                                        mappings.insert(rel_pn.to_string(),
                                            str::from_utf8(cv.value.as_ref()).unwrap().to_string());
                                    }
                                }
                            },
                            _ => {}
                        }
                    }
                },
                Ok(Event::End(e)) => {
                    if e.local_name().as_ref() == b"Types" {
                        in_types = false;
                    }
                }
                _ => ()
            }
        }

        (defaults, mappings)
    }

    fn get_files_with_content_type(dir: &str, content_type: &str) -> (HashMap<String, String>, Vec<String>) {
        let (defaults, mappings) = Self::get_content_types(dir);

        let mut result = vec!();
        for (file, ct) in &mappings {
            if *ct == content_type {
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


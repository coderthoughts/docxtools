use quick_xml::events::{Event, BytesStart, BytesText};
use quick_xml::events::attributes::{Attr, Attribute, Attributes};
use quick_xml::name::QName;
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use regex::Regex;
use std::collections::{BTreeMap, HashMap};
use std::fs::{File, self};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf, MAIN_SEPARATOR, MAIN_SEPARATOR_STR};
use std::str;
use uuid::Uuid;
use unicase::UniCase;
use walkdir::WalkDir;

use crate::file_util::FileUtil;
use crate::zip_util::ZipUtil;

#[cfg(windows)]
const LINE_ENDING: &'static str = "\r\n";
#[cfg(not(windows))]
const LINE_ENDING: &'static str = "\n";

const WORDDOC_MT: &'static str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml";
const RELATION_MT: &'static str = "application/vnd.openxmlformats-package.relationships+xml";
const WORDDOC_NS: &'static str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

#[derive(Clone, Debug)]
enum Mode {
    // Output the value of a certain attribute
    CatAttrCondition {
        tagname: String, // The tag name the attribute should be on
        attrname: String, // The name of the attribute
        condkey: String, // Another attribute that should be there
        condval: String, // The value this other attribute should have
    },
    Cat,
    Grep {
        regex: Regex
    },
    Replace {
        regex: Regex,
        replacement: String
    },
    ReplaceAttribute {
        regex: Regex,
        replacement: String
    },
    StyleChange {
        style: String,
        replacement: String
    }
}

pub struct XMLUtil {
}

/// A collection of functions for working with .docx XML files. The functions generally expect that the .docx file
/// is already unzipped and passed in as the root directory of the location where is was unzipped.
impl XMLUtil {
    /// Send the text content of the docx structure to stdout. `dir` is the directory containing
    /// the unzipped docx file and `src_file` is the original name of the docx file.
    pub fn cat(dir: &str, src_file: &str) {
        Self::snr_xml(Mode::Cat, dir, src_file, None, None);
    }

    pub fn cat_rel_attr(el_name: &str, attr_name: &str, cond_key: &str, cond_val: &str,
            dir: &str, src_file: &str) {
        let fref = Self::get_rel_files(dir);

        let mode = Mode::CatAttrCondition {
                tagname: el_name.into(), attrname: attr_name.into(),
                condkey: cond_key.into(), condval: cond_val.into()
            };
        Self::snr_xml(mode, dir, src_file, Some(fref),None);
    }

    pub fn change_style(dir: &str, src_file: &str, style: &str, replacement: &str, output_file: &Option<&str>) {
        Self::invoke_with_files_and_output(dir, src_file, output_file, |files, out_file| {
            let styles = Self::get_all_styles(dir, src_file);

            let src = styles.get(&UniCase::new(style.to_string()));
            let dest = styles.get(&UniCase::new(replacement.to_string()));

            if let (Some(src_id), Some(dest_id)) = (src, dest) {
                let mode = Mode::StyleChange {
                    style: src_id.clone(), replacement: dest_id.clone()
                };
                Self::snr_xml(mode, dir, src_file, files, out_file);
            } else {
                let mut style_names = Vec::from_iter(styles.keys());
                style_names.sort();

                panic!("Not all styles were found. Known styles (case insensitive): {:?}", style_names);
            }
        });
    }

    /// Search for regex `pattern` in the text of the docx structure and send matches to stdout.
    /// `dir` is the directory containing
    /// the unzipped docx file and `src_file` is the original name of the docx file.
    pub fn grep_xml(dir: &str, src_file: &str, pattern: &str) {
        let mg = Mode::Grep { regex: Regex::new(pattern).expect(pattern) };
        Self::snr_xml(mg, dir, src_file, None, None);
    }

    /// Search for regex `pattern` in the text of the docx structure and replace all occurrences with `replacement`.
    /// `dir` is the directory containing
    /// the unzipped docx file and `src_file` is the original name of the docx file.
    ///
    /// `output_file` can be a .docx filename. If specified the result will be zipped and written to produce this
    /// new .docx file. Otherwise the result is zipped and written to `src_file`.
    pub fn replace_xml(dir: &str, src_file: &str, pattern: &str, replacement: &str, output_file: &Option<&str>) {
        Self::invoke_with_files_and_output(dir, src_file, output_file, |files, out_file| {
            let mr = Mode::Replace {
                regex: Regex::new(pattern).unwrap(),
                replacement: replacement.to_owned()
            };
            Self::snr_xml(mr, dir, src_file, files, out_file)
        });
    }

    pub fn replace_rel_attr(dir: &str, src_file: &str, pattern: &str, replace: &str, output_file: &Option<&str>) {
        let fref = Self::get_rel_files(dir);

        let out_file = match output_file {
            Some(of) => of,
            None => src_file
        };

        let ma = Mode::ReplaceAttribute {
            regex: Regex::new(pattern).unwrap(),
            replacement: replace.to_owned()
        };
        Self::snr_xml(ma, dir, src_file, Some(fref), Some(out_file));
    }

    fn invoke_with_files_and_output<'a, F>(dir: &str, src_file: &'a str, output_file: &Option<&'a str>, op_fn: F)
        where F: Fn(Option<Vec<String>>, Option<&'a str>) {

        let out_file = match output_file {
            Some(of) => of,
            None => src_file
        };

        let (_, files) = Self::get_files_with_content_type(dir, WORDDOC_MT);

        op_fn(Some(files), Some(out_file));
    }

    fn get_rel_files(dir: &str) -> Vec<String> {
        let (defaults, files) = Self::get_files_with_content_type(dir, WORDDOC_MT);
        let rels_extension = &defaults[RELATION_MT];

        let mut rels_files = vec!();
        for f in files {
            let last_slash = f.rfind(MAIN_SEPARATOR).expect(&f);
            let mut new_fn = String::new();
            new_fn.push_str(&f[..last_slash]);
            new_fn.push_str("/_");
            new_fn.push_str(rels_extension);
            new_fn.push_str(&f[last_slash..]);
            new_fn.push('.');
            new_fn.push_str(rels_extension);
            rels_files.push(FileUtil::normalize_path(&new_fn));
        }

        rels_files
    }

    /// Iterate recursively over all files in `dir` and perform the operation specified in `mode` on each file. The original name
    /// of the .docx file is provided in `src_file`.
    ///
    /// Optionally specify `files` as the list of files to match. If not specified, all files ending with `.xml` are matched.
    /// `pattern` and `replacement` are used to search/replace operations.
    /// `output_file` optionally specifies a different output file for replacement operations.
    fn snr_xml(mode: Mode, dir: &str, src_file: &str, files: Option<Vec<String>>, output_file: Option<&str>) {
        let mut base_dir = FileUtil::normalize_path(dir);
        if !base_dir.ends_with(MAIN_SEPARATOR_STR) {
            base_dir.push(MAIN_SEPARATOR);
        }

        for entry in WalkDir::new(dir).into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file()) {
            let sub_path = FileUtil::get_sub_path(entry.path(), &base_dir);

            if let Some(file_list) = &files {
                if !file_list.contains(&sub_path) {
                    continue;
                }
            } else {
                if !(sub_path.ends_with(".xml")) {
                    continue;
                }
            }

            Self::snr_xml_file(&mode, entry.path(), src_file);
        }

        if let Some(outfile) = output_file {
            ZipUtil::write_zip(dir, outfile).expect(outfile);
        }
    }

    fn snr_xml_file(mode: &Mode, path: &Path, src_file: &str) {
        match mode {
            Mode::Cat =>
                Self::cat_text(path, src_file),
            Mode::ReplaceAttribute { regex, replacement } =>
                Self::snr_change_attribute(path, regex, replacement, src_file),
            Mode::CatAttrCondition { .. } =>
                Self::cat_xml_attribute(mode, path, src_file),
            Mode::Grep { regex } =>
                Self::grep_text(path, src_file, regex),
            Mode::Replace { regex, replacement } =>
                Self::replace_text(path, src_file, regex, replacement),
            Mode::StyleChange { style, replacement } =>
                Self::style_change(path, src_file, style, replacement)
        }
    }

    fn get_reader(path: &Path) -> Reader<BufReader<File>> {
        Reader::from_file(path).expect(&path.to_string_lossy())
    }

    fn read_namespaces(e: &BytesStart, nslist: &mut Vec<String>) {
        if nslist.len() != 1 {
            panic!("Should contain exactly 1 root namespace: {:?}", nslist);
        }
        let initial_namespace = nslist.get(0).unwrap().to_owned();

        for attr in e.attributes() {
            if let Ok(a) = attr {
                let k = str::from_utf8(a.key.as_ref());
                if let Ok(key) = k {
                    if key.starts_with("xmlns:") {
                        if a.value.as_ref() == initial_namespace.as_bytes() {
                            let alt_name = (&key[6..]).to_string();
                            nslist.push(alt_name);
                        }
                    }
                }
            }
        }
    }

    /// Writes the `writer` to disk which is assumed to write to `temp_file`. If `has_changes` is `true`
    /// then replaces the `xml_file` with `temp_file`, otherwise just deletes `temp_file`.
    fn finish_writing(writer: Writer<BufWriter<File>>, xml_file: &Path, temp_file: &Path, has_changes: bool) {
        // This writes out the file
        writer.into_inner().into_inner().unwrap();

        if has_changes {
            // Replace the original file with the new one.
            fs::remove_file(xml_file).unwrap();
            fs::rename(temp_file, xml_file).unwrap();
        } else {
            // No changes, so just remove the generated file.
            fs::remove_file(temp_file).unwrap();
        }
    }

    /// For each namespace in `nsl` produce a qname result that has the namespace and `tag` as local name.
    fn nsl_to_fqnames(nsl: &Vec<String>, tag: &str) -> Vec<String> {
        let mut fqnames = vec![];

        for ns in nsl {
            let mut fq = ns.clone();
            fq.push(':');
            fq.push_str(tag);
            fqnames.push(fq);
        }

        fqnames
    }

    /// Convert a list of qnames specified as strings in to a list of QNames.
    fn qnames(fqnl: &Vec<String>) -> Vec<QName> {
        let mut qnames = vec![];

        for fqn in fqnl {
            qnames.push(QName(fqn.as_bytes()));
        }

        qnames
    }

    /// Check if any of the namespaces specified as `nslist` with `tag` as local name contains the QName
    /// specified as `qn`.
    fn match_tag(qn: &QName, nslist: &Vec<String>, tag: &str) -> bool {
        let para_fqnl = Self::nsl_to_fqnames(nslist, tag);
        let para_qnames = Self::qnames(&para_fqnl);
        let contains = para_qnames.contains(qn);
        contains
    }

    fn create_temp_file(dir: &Path) -> PathBuf {
        let mut temp_res = dir.to_owned();
        temp_res.push(format!("{}.xml", Uuid::new_v4()));

        temp_res
    }

    /// Read the contents of `xml_file` which would typically be a `word/document.xml` file and collect
    /// all paragraphs of text in the result as a `Vec<String>`.
    ///
    /// In the input XML file a single paragraph and even a single word might be spread over different
    /// <w:t> tags. The String list returned merges these together so that the result looks like what
    /// you would see in the word processor. However, in order to replace text, we need to know which text
    /// originated in which tag. For this this method numbers the <w:t> tags in the document and in its
    /// second return value it returns a BTreeMap where the key is the number, or id, of each text element
    /// and the value is a tuple where the first value is the paragraph that is relates to and the second
    /// value is the character position in that paragraph that the tag with this id starts.
    ///
    /// `src_file` is the name of the original .docx file.
    /// If the `replacements` HashMap contains data, then these will be applied and the result is used to
    /// overwrite the `xml_file` input file.
    /// The keys of the `replacements` map is the id of the tags that need to be replaced and the first
    /// value of the value tuple of `replacements` is the new value for this tag. The second value of the
    /// tuple is not used in this function.
    fn get_replace_text(xml_file: &Path, src_file: &str, replacements: HashMap<usize, (String, Vec<i32>)>)
            -> (Vec<String>, BTreeMap<usize, (usize, usize)>) {
        let mut reader = Self::get_reader(xml_file);

        let temp_file = Self::create_temp_file(xml_file.parent().unwrap());
        let tf = File::create(&temp_file).unwrap();
        let mut writer = Writer::new(BufWriter::new(tf));

        let mut paras = Vec::new();
        let mut cur_line = String::new();
        let mut coords = BTreeMap::new();

        let mut buf = Vec::new();

        let mut nslist = vec![WORDDOC_NS.to_string()];

        let mut first_element = true;
        let mut inside_paragraph = false;
        let mut inside_text = false;
        let mut text_els: usize = 0;
        loop {
            let ev = reader.read_event_into(&mut buf);
            // println!("Read event: {:?}", ev);
            match ev {
                Err(e) => panic!("Error reading {} at position {}: {:?}", src_file, reader.buffer_position(), e),
                Ok(Event::Eof) => break,
                Ok(Event::Empty(e)) => {
                    if Self::match_tag(&e.name(), &nslist, "br") {
                        cur_line.push_str(LINE_ENDING);
                    }
                    writer.write_event(Event::Empty(e)).expect(&temp_file.to_string_lossy());
                },
                Ok(Event::Start(e)) => {
                    if first_element {
                        first_element = false;
                        Self::read_namespaces(&e, &mut nslist);
                    }

                    if Self::match_tag(&e.name(), &nslist, "p") {
                        inside_paragraph = true;
                    } else if inside_paragraph && Self::match_tag(&e.name(), &nslist, "t") {
                        inside_text = true;
                    } else if inside_paragraph && Self::match_tag(&e.name(), &nslist, "br") {
                        cur_line.push_str(LINE_ENDING);
                    }
                    writer.write_event(Event::Start(e)).expect(&temp_file.to_string_lossy());
                },
                Ok(Event::End(e)) => {
                    if Self::match_tag(&e.name(), &nslist, "p") {
                        inside_paragraph = false;
                        if cur_line.len() > 0 {
                            paras.push(cur_line.clone());
                        }
                        cur_line.clear();
                    } else if inside_paragraph && Self::match_tag(&e.name(), &nslist, "t") {
                        inside_text = false;
                    }
                    writer.write_event(Event::End(e)).expect(&temp_file.to_string_lossy());
                },
                Ok(Event::Text(t)) => {
                    let mut ct = t;

                    if inside_text {
                        let val = ct.unescape().expect(src_file);
                        if val.len() > 0 {
                            coords.insert(text_els, (paras.len(), cur_line.len()));

                            let new_text = replacements.get(&text_els);
                            if let Some((nt, _)) = new_text {
                                ct = BytesText::new(nt);
                                println!("{}: {}\n-> {}", src_file, val, nt);
                            }

                            text_els += 1;

                            cur_line.push_str(val.as_ref());
                        }
                    }
                    writer.write_event(Event::Text(ct)).expect(&temp_file.to_string_lossy());
                },
                Ok(e) => writer.write_event(e).expect(&temp_file.to_string_lossy())
            }
        }

        drop(reader); // Close the file being read

        Self::finish_writing(writer, xml_file, &temp_file, !replacements.is_empty());

        (paras, coords)
    }

    fn cat_text(path: &Path, src_file: &str) {
        let (paras, _) = Self::get_replace_text(path, src_file, HashMap::new());

        for para in paras {
            println!("{}: {}", src_file, para);
        }
    }

    fn grep_text(path: &Path, src_file: &str, regex: &Regex) {
        let (paras, _) = Self::get_replace_text(path, src_file, HashMap::new());

        for para in paras {
            if regex.is_match(&para) {
                println!("{}: {}", src_file, para);
            }
        }
    }

    fn get_line_coords(cur_line: usize, coords: &BTreeMap<usize, (usize, usize)>) -> BTreeMap<usize, (usize, usize)> {
        let mut res = BTreeMap::new();

        for (id, (line, pos)) in coords {
            if cur_line == *line {
                res.insert(*pos, (usize::MAX, *id));

                if *pos > 0 {
                    // The position is not at the start of the line, update the previous position with the endpos
                    let prev_id = id - 1;
                    let prev = coords.get(&prev_id);

                    if let Some((pl, ppos)) = prev {
                        if *pl != cur_line {
                            // Shouldn't happen
                            break;
                        }
                        res.insert(*ppos, (*pos, prev_id));
                    }
                }
            }
        }

        res
    }

    /// Apply the replacements needed for a single <w:t> tag. The id (internal number) of the tag is
    /// provided in `tag_id`. The original contents of the tags is provided in `tag`. The replacement text
    /// is in `replace` and the location start and end of the text in the original tag to be replaced
    /// is in `match_start` and `match_end`.
    ///
    /// `replacements` contains the currently known set of replacements, which may already contain other
    /// replacements made for the current tag. The key of this HashMap is the tag id and the value contains
    /// the current value of the tag, given any previous replacements and an offset mapping that contains
    /// for each character position in the original tag text a positive or negative offset in case the
    /// match locations must be adjusted given any previously applied replacements, as they may have
    /// changed the length of text in the tag.
    fn replace_within_tag(replacements: &mut HashMap<usize, (String, Vec<i32>)>, tag_id: usize, tag: &str,
            match_start: usize, match_end: usize, replace: &str) {
        let prev_repl = replacements.get(&tag_id);

        let mut replaced;
        let mut corr_idxs: Vec<i32>;
        if let Some((r, c)) = prev_repl {
            replaced = r.clone();
            corr_idxs = c.clone();
        } else {
            replaced = tag.to_string();
            corr_idxs = vec![0; replaced.len()];
        }

        let mut correction = 0;
        for i in 0..match_start {
            correction += corr_idxs[i as usize];
        }

        let repl_start = (match_start as i32 + correction) as usize;
        let repl_end = (match_end as i32 + correction) as usize;
        replaced.replace_range(repl_start..repl_end, replace);

        let delta = replace.len() as i32 - match_end as i32 + match_start as i32;
        if delta < 0 {
            let from_pos = (match_end as i32) + correction;
            for i in from_pos+delta .. from_pos {
                corr_idxs[i as usize] -= 1;
            }
        } else if delta > 0 {
            let corr_pos = (match_end as i32) - 1;
            corr_idxs[corr_pos as usize] += delta;
        }

        replacements.insert(tag_id, (replaced, corr_idxs));
    }

    /// In the file pointed to by `path` replace all matching `regex`es with the `replace` value.
    /// The input file will be overwritten with the result. `src_file` is the name of the original
    /// .docx file
    ///
    /// This method works by reading the file contents first via `get_replace_text` and applying the
    /// regex replacements to its result (a list of strings, representing each paragraph).
    ///
    /// Replacements are mapped to <w:t> tags which are numbered internally.
    /// Once all the replacements have been found, the `get_replace_text` method is called again
    /// but now with the replacements to-be-applied.
    fn replace_text(path: &Path, src_file: &str, regex: &Regex, replace: &str) {
        let mut replacements: HashMap<usize, (String, Vec<i32>)> = HashMap::new();

        let (paras, coords) = Self::get_replace_text(path, src_file, HashMap::new());

        let mut cur_line: usize = 0;
        for para in paras {
            let line_coords = Self::get_line_coords(cur_line, &coords);
            for m in regex.find_iter(&para) {
                let mstart = m.start();
                let mend = m.end();

                let mut start_id = 0;
                let mut end_id = 0;
                let mut start_idx = 0;

                let mut tags = BTreeMap::new();
                for (idx, (eidx, id)) in &line_coords {
                    let neidx;
                    if *eidx > para.len() {
                        neidx = para.len();
                    } else {
                        neidx = *eidx;
                    }

                    let t = &para[*idx..neidx];
                    tags.insert(*id, t);

                    if *idx <= mstart {
                        start_id = *id;
                        end_id = *id;
                        start_idx = *idx;
                    }
                    if *idx < mend {
                        end_id = *id;
                    }
                }

                // The match region is between start_id and end_id now

                if start_id == end_id {
                    // simplest case start and end are the same:
                    if let Some(tag) = tags.get(&start_id) {
                        Self::replace_within_tag(&mut replacements, start_id, tag, mstart - start_idx, mend - start_idx, replace);
                    }
                } else {
                    /*
                        1. get the length of the replacement
                        2. get all tags
                        3. Walk over tags, first one from match position, later ones from start
                        4. divide up the caracters:
                            all up to but not including last:
                                replace the characters
                            last one:
                                replace the rest
                    */

                    let mut remaining_chars = mend as i32 - mstart as i32;
                    let mut cur_replacement = replace.to_string();
                    for i in start_id..end_id + 1 {
                        if remaining_chars < 0 { remaining_chars = 0; }

                        if let Some(tag) = tags.get(&i) {
                            if i == start_id {
                                let chars = tag.len() - mstart;

                                let repl;
                                if cur_replacement.len() >= chars {
                                    repl = &cur_replacement[0..chars];
                                } else {
                                    repl = &cur_replacement;
                                }
                                Self::replace_within_tag(&mut replacements, i, tag, mstart, mstart + chars, repl);

                                remaining_chars -= chars as i32;
                                if cur_replacement.len() >= chars {
                                    cur_replacement = cur_replacement[chars..].to_string();
                                } else {
                                    cur_replacement.clear();
                                }
                            } else if i == end_id {
                                Self::replace_within_tag(&mut replacements, i, tag, 0, remaining_chars as usize, &cur_replacement);
                            } else {
                                let repl;
                                if cur_replacement.len() >= tag.len() {
                                    repl = &cur_replacement[0..tag.len()];
                                } else {
                                    repl = &cur_replacement;
                                }

                                Self::replace_within_tag(&mut replacements, i, tag, 0, tag.len(), repl);

                                remaining_chars -= tag.len() as i32;
                                if cur_replacement.len() >= tag.len() {
                                    cur_replacement = cur_replacement[tag.len()..].to_string();
                                } else {
                                    cur_replacement.clear();
                                }
                            }
                        }
                    }
                }
            }
            cur_line += 1;
        }

        if !replacements.is_empty() {
            Self::get_replace_text(path, src_file, replacements);
        }
    }

    fn style_change(xml_file: &Path, src_file: &str, style: &str, replace: &str) {
        let mut reader = Self::get_reader(xml_file);

        let temp_file = Self::create_temp_file(xml_file.parent().unwrap());
        let tf = File::create(&temp_file).unwrap();
        let mut writer = Writer::new(BufWriter::new(tf));

        let mut has_changes = false;
        let mut buf = Vec::new();
        let mut nslist = vec![WORDDOC_NS.to_string()];
        let mut first_element = true;
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(mut e)) => {
                    if Self::match_tag(&e.name(), &nslist, "pStyle") {
                        let (updated, c) = Self::update_attribute(e, "val", style, replace, src_file);
                        has_changes |= c;
                        e = updated;
                    }
                    writer.write_event(Event::Empty(e)).expect(&temp_file.to_string_lossy());
                },
                Ok(Event::Start(mut e)) => {
                    if first_element {
                        first_element = false;
                        Self::read_namespaces(&e, &mut nslist);
                    }
                    if Self::match_tag(&e.name(), &nslist, "pStyle") {
                        let (updated, c) = Self::update_attribute(e, "val", style, replace, src_file);
                        has_changes |= c;
                        e = updated;
                    }
                    writer.write_event(Event::Start(e)).expect(&temp_file.to_string_lossy());
                },
                Ok(Event::Eof) => break,
                Ok(e) => writer.write_event(e).unwrap(),
                Err(e) => panic!("Error {:?}", e),
            }
        }

        Self::finish_writing(writer, xml_file, &temp_file, has_changes);
    }

    fn snr_change_attribute(xml_file: &Path, regex: &Regex, replace: &str, src_file: &str) {
        let mut reader = Self::get_reader(xml_file);

        let mut has_changes = false;
        let mut buf = Vec::new();

        let temp_file = Self::create_temp_file(xml_file.parent().unwrap());
        let tf = File::create(&temp_file).unwrap();
        let mut writer = Writer::new(BufWriter::new(tf));
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(e)) => {
                    let (update_attributes, c) = Self::update_any_attribute(e, &regex, replace, src_file);
                    has_changes |= c;
                    writer.write_event(Event::Empty(update_attributes)).unwrap();
                },
                Ok(Event::Start(e)) => {
                    let (update_attributes, c) = Self::update_any_attribute(e, &regex, replace, src_file);
                    has_changes |= c;
                    writer.write_event(Event::Start(update_attributes)).unwrap();
                },
                Ok(Event::Eof) => break,
                Ok(e) => writer.write_event(e).unwrap(),
                Err(e) => panic!("Error {:?}", e),
            }
        }

        Self::finish_writing(writer, xml_file, &temp_file, has_changes);
    }

    fn update_attribute<'a>(bs: BytesStart<'a>, attr_name: &str, search: &str, replace: &str, src_file: &str) -> (BytesStart<'a>, bool) {
        let prefix = bs.name().prefix();

        if let Some(pfx) = prefix {
            let attr_key = format!("{}:{}", str::from_utf8(pfx.as_ref()).unwrap(), attr_name);
            let qnattr = QName(attr_key.as_bytes());

            let rex = Regex::new(&format!("^{}$", search)).expect(search);
            Self::update_attributes(bs, Some(qnattr), &rex, replace, src_file)
        } else {
            (bs, false)
        }
    }

    fn update_any_attribute<'a>(bs: BytesStart<'a>, regex: &Regex, replace: &str, src_file: &str) -> (BytesStart<'a>, bool) {
        Self::update_attributes(bs, None, regex, replace, src_file)
    }

    fn update_attributes<'a>(bs: BytesStart<'a>, attr_name: Option<QName>, regex: &Regex, replace: &str, src_file: &str) -> (BytesStart<'a>, bool) {
        let mut es = bs.clone();

        es.clear_attributes();

        let mut changed = false;
        for attr in bs.attributes() {
            if let Ok(a) = attr {
                let key = a.key;
                if let Some(ak) = attr_name {
                    if ak != key {
                        // There is a key QName specified and they don't match, go to the next attribute
                        continue;
                    }
                }

                let val = str::from_utf8(&a.value);

                if let Ok(v) = val {
                    let mut rval = v;
                    let rv;
                    if regex.is_match(&v) {
                        println!("{}: {}={}", src_file, str::from_utf8(a.key.as_ref()).unwrap(), v);
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

    fn cat_xml_attribute(mode: &Mode, path: &Path, src_file: &str) {
        let mut reader = Self::get_reader(path);
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Err(e) => panic!("Error reading {} at position {}: {:?}", src_file, reader.buffer_position(), e),
                Ok(Event::Eof) => break,
                Ok(Event::Empty(e)) |
                Ok(Event::Start(e)) => {
                    if let Mode::CatAttrCondition { tagname, attrname, condkey, condval } = mode {
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
                                        mappings.insert(FileUtil::normalize_path(rel_pn),
                                            str::from_utf8(cv.value.as_ref()).unwrap().to_string());
                                    }
                                }
                            },
                            _ => ()
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

    fn get_attribute(attrs: &mut Attributes, nslist: &Vec<String>, name: &str) -> String {
        for attr in attrs {
            if let Ok(a) = attr {
                let key = a.key;
                if Self::match_tag(&key, nslist, name) {
                    let val = str::from_utf8(&a.value);
                    if let Ok(v) = val {
                        return v.to_string()
                    }
                }
            }
        }

        panic!("Attribute with name {} not found", name);
    }

    /// Returns a map from display name to styleId
    fn get_all_styles(dir: &str, src_file: &str) -> HashMap<UniCase<String>, String> {
        let mut style_map = HashMap::new();

        let styles_file_name = &format!("{}{}", dir, "/word/styles.xml");
        let styles_path = Path::new(styles_file_name);
        let mut reader = Self::get_reader(&styles_path);

        let mut buf = Vec::new();
        let mut nslist = vec!(WORDDOC_NS.to_string());
        let mut inside_style = String::new();
        let mut first_element = true;
        loop {
            match reader.read_event_into(&mut buf) {
                Err(e) => panic!("Error reading {} from {} at position {}: {:?}",
                    styles_file_name, src_file, reader.buffer_position(), e),
                Ok(Event::Eof) => break,
                Ok(Event::Empty(e)) => {
                    if inside_style.len() > 0 && Self::match_tag(&e.name(), &nslist, "name") {
                        let display_name = Self::get_attribute(&mut e.attributes(), &nslist, "val");
                        style_map.insert(UniCase::new(display_name), inside_style.to_owned());
                    }
                },
                Ok(Event::Start(e)) => {
                    if first_element {
                        first_element = false;
                        Self::read_namespaces(&e, &mut nslist);
                    }

                    if Self::match_tag(&e.name(), &nslist, "style") {
                        let style_id = Self::get_attribute(&mut e.attributes(), &nslist, "styleId");
                        inside_style.clear();
                        inside_style.push_str(style_id.as_str());
                    } else if inside_style.len() > 0 && Self::match_tag(&e.name(), &nslist, "name") {
                        let display_name = Self::get_attribute(&mut e.attributes(), &nslist, "val");
                        style_map.insert(UniCase::new(display_name), inside_style.to_owned());
                    }
                },
                Ok(Event::End(e)) => {
                    if Self::match_tag(&e.name(), &nslist, "style") {
                        inside_style.clear();
                    }
                }
                _ => ()
            }
        }

        style_map
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

    #[test]
    #[serial]
    fn test_cat2() {
        let out = capture_stdout!(XMLUtil::cat("./src/test/test_tree5", "wordbreak.docx"));
        let expected =
            "wordbreak.docx: Notwithstanding the eventual resulting quotations punters were agreeable to a technocratic compromise.".to_string()
            + super::LINE_ENDING + "Here’s another line of text.";
        let idx1 = out.find(&expected).unwrap();
        let idx2 = out.find("wordbreak.docx: And this text is in the next paragraph.").unwrap();

        assert!(idx1 < idx2);
    }

    #[test]
    #[serial] // This test has to run serially to avoid multiple tests to capture stdout
    fn test_grep() {
        let out = capture_stdout!(XMLUtil::grep_xml("./src/test/test_tree2", "doc123.docx", "[oe]re"));
        println!("out: {}", out);
        assert!(out.contains("doc123.docx: And some some some more text"));
        assert!(out.contains("doc123.docx: Something here"));
        assert!(out.contains("doc123.docx: Here’s a hyperlink:"));
        assert!(out.contains("doc123.docx: And here’s just some text:"));
        assert!(!out.contains("Target"));
    }

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

    #[test]
    fn test_replace_shorten() -> io::Result<()> {
        let orgdir = "./src/test/test_tree2";
        let testdir = testdir!();

        copy_dir_all(orgdir, &testdir)?;

        let before = fs::read_to_string("./src/test/test_tree2/word/document.xml")?;
        assert!(before.contains("And some some some more text"), "Precondition");
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
        assert!(after.contains("And zzz zzz zzz more text"));
        assert!(after.contains("and then zzz"));
        assert!(after.contains("zzzthing here"));
        assert!(after.contains(">zzz"));
        assert!(!after.contains("some"));
        assert!(!after.contains("Some"));

        Ok(())
    }

    #[test]
    fn test_replace_make_longer() -> io::Result<()> {
        let orgdir = "./src/test/test_tree2";
        let testdir = testdir!();

        copy_dir_all(orgdir, &testdir)?;

        let before = fs::read_to_string("./src/test/test_tree2/word/document.xml")?;
        assert!(before.contains("And some some some more text"), "Precondition");
        assert!(before.contains("and then some"), "Precondition");
        assert!(before.contains("Something here"), "Precondition");
        assert!(before.contains(">some<"), "Precondition");
        assert!(before.contains(">Some <"), "Precondition");
        assert!(!before.contains("zzz"), "Precondition");

        XMLUtil::replace_xml(&testdir.to_string_lossy(), "my-source.docx",
            "[Ss]ome", "ABCDEF",
            &Some(&testdir.join("output.docx").to_string_lossy()));

        // Check that the replacement worked as expected
        let after = fs::read_to_string(testdir.join("word/document.xml"))?;
        assert!(after.contains("And ABCDEF ABCDEF ABCDEF more text"));
        assert!(after.contains("and then ABCDEF"));
        assert!(after.contains("ABCDEFthing here"));
        assert!(after.contains(">ABCDEF"));
        assert!(!after.contains("some"));
        assert!(!after.contains("Some"));

        Ok(())
    }

    #[test]
    fn test_replace_across_tags() -> io::Result<()> {
        let orgdir = "./src/test/test_tree5";
        let testdir = testdir!();

        copy_dir_all(orgdir, &testdir)?;

        let before = fs::read_to_string("./src/test/test_tree5/word/document.xml")?;
        assert!(before.contains("re"), "Precondition");
        assert!(before.contains("sult"), "Precondition");
        assert!(before.contains("ing"), "Precondition");
        assert!(!before.contains("resulting"), "Precondition");

        XMLUtil::replace_xml(&testdir.to_string_lossy(), "acrstags.docx",
            "resulting", "1234567890",
            &Some(&testdir.join("output.docx").to_string_lossy()));

        let after = fs::read_to_string(testdir.join("word/document.xml"))?;
        assert!(after.contains("eventual 12<"));
        assert!(after.contains(">3456<"));
        assert!(after.contains(">7890 quotations"));
        assert!(!after.contains("1234567890"));

        Ok(())
    }

    #[test]
    fn test_replace_across_tags0() -> io::Result<()> {
        let orgdir = "./src/test/test_tree5";
        let testdir = testdir!();

        copy_dir_all(orgdir, &testdir)?;

        let before = fs::read_to_string("./src/test/test_tree5/word/document.xml")?;
        assert!(before.contains("re"), "Precondition");
        assert!(before.contains("sult"), "Precondition");
        assert!(before.contains("ing"), "Precondition");
        assert!(!before.contains("resulting"), "Precondition");

        XMLUtil::replace_xml(&testdir.to_string_lossy(), "acrstags.docx",
            "resulting", "1",
            &Some(&testdir.join("output.docx").to_string_lossy()));

        let after = fs::read_to_string(testdir.join("word/document.xml"))?;
        assert!(after.contains("eventual 1<"));
        assert!(after.contains("><"));
        assert!(after.contains("> quotations"));

        Ok(())
    }

    #[test]
    fn test_replace_across_tags1() -> io::Result<()> {
        let orgdir = "./src/test/test_tree5";
        let testdir = testdir!();

        copy_dir_all(orgdir, &testdir)?;

        let before = fs::read_to_string("./src/test/test_tree5/word/document.xml")?;
        assert!(before.contains("re"), "Precondition");
        assert!(before.contains("sult"), "Precondition");
        assert!(before.contains("ing"), "Precondition");
        assert!(!before.contains("resulting"), "Precondition");

        XMLUtil::replace_xml(&testdir.to_string_lossy(), "acrstags.docx",
            "resulting", "123",
            &Some(&testdir.join("output.docx").to_string_lossy()));

        let after = fs::read_to_string(testdir.join("word/document.xml"))?;
        assert!(after.contains("eventual 12<"));
        assert!(after.contains(">3<"));
        assert!(after.contains("> quotations"));

        Ok(())
    }

    #[test]
    fn test_replace_across_tags2() -> io::Result<()> {
        let orgdir = "./src/test/test_tree2";
        let testdir = testdir!();

        copy_dir_all(orgdir, &testdir)?;

        XMLUtil::replace_xml(&testdir.to_string_lossy(), "xyz.docx",
            "(text and|then some)", "aaa", &None);

        let after = fs::read_to_string(testdir.join("word/document.xml"))?;
        assert!(after.contains("some more aaa</w:t"));
        assert!(after.contains("> aaa<"));

        Ok(())
    }

    #[test]
    fn test_replace_across_tags3() -> io::Result<()> {
        let orgdir = "./src/test/test_tree2";
        let testdir = testdir!();

        copy_dir_all(orgdir, &testdir)?;

        XMLUtil::replace_xml(&testdir.to_string_lossy(), "xyz.docx",
            "(text and|then some)", "bbbbb", &None);

        let after = fs::read_to_string(testdir.join("word/document.xml"))?;
        assert!(after.contains("some more bbbb</w:t"));
        assert!(after.contains(">b bbbbb<"));

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

        XMLUtil::replace_rel_attr(&testdir.to_string_lossy(), "my-source.docx",
            "www.example.com", "foobar.org",
            &Some(&testdir.join("output-2.docx").to_string_lossy()));

        let after_doc = fs::read_to_string(testdir.join("word/document.xml"))?;
        let after = fs::read_to_string(testdir.join("word/_rels/document.xml.rels"))?;

        assert!(after.contains("Target=\"http://foobar.org/\""));
        assert!(after_doc.contains(">www.example.com<"), "Should not have changed the document text");

        Ok(())
    }

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
        println!("After: {}", after);
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

    #[test]
    fn test_change_style() -> io::Result<()> {
        let orgdir = "./src/test/test_tree6";
        let testroot = testdir!();
        let testdir = testroot.join("subdir");

        copy_dir_all(orgdir, &testdir)?;

        let before = fs::read_to_string("./src/test/test_tree6/word/document.xml")?;
        let match_indexes: Vec<_> = before.match_indices("Heading1").map(|(i, _)| i).collect();

        assert!(before.contains("Heading1"), "Precondition");
        XMLUtil::change_style(&testdir.to_string_lossy(), "headings.docx",
            "Heading 1", "Heading 3", &None);

        let after = fs::read_to_string(testdir.join("word/document.xml"))?;

        for idx in match_indexes {
            let new_val = &after[idx..idx+"Heading1".len()];
            assert_eq!("Heading3", new_val);
        }
        assert_eq!(3, after.match_indices("Heading3").count());

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


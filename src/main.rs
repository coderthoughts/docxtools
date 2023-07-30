// use docx_rs::*;
use std::path::PathBuf;
use std::fs::File;
use std::io::Read;

use docx_rs::*;

fn read_to_vec(file_name: &PathBuf) -> anyhow::Result<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();
    let mut f = File::open(file_name)?;
    f.read_to_end(&mut buf)?;
    Ok(buf)
}

fn process_para(p: &Paragraph) {
    println!("Para: {:?}", p.raw_text());
}

fn main() {
    let path = PathBuf::from("docs/original.docx");
    let original_docx = docx_rs::read_docx(&read_to_vec(&path).unwrap()).unwrap();

    let doc = original_docx.document;
    for c in doc.children {
        match c {
            DocumentChild::Paragraph(p) => process_para(&p),
            _ => ()
        }
        // println!("c: {:?}", c);
    }

    // println!("doc: {:?}", original_docx);
}

// use clap::Parser;
// use docx_rs::*;
// use serde_json::Value;
// use std::io::Read;

// #[derive(Parser, Debug)]
// #[command(author, version, about, long_about = None)]
// struct Args {
//     #[arg(short, long)]
//     name: String,
// }

// fn parse_docx(file_name: &str) -> anyhow::Result<()> {
//     let data: Value = serde_json::from_str(&read_docx(&read_to_vec(file_name)?)?.json())?;
//     if let Some(children) = data["document"]["children"].as_array() {
//         children.iter().for_each(read_children);
//     }
//     Ok(())
// }

// fn read_children(node: &Value) {
//     if let Some(children) = node["data"]["children"].as_array() {
//         children.iter().for_each(|child| {
//             if child["type"] != "text" {
//                 read_children(child);
//             } else {
//                 println!("{}", child["data"]["text"]);
//             }
//         });
//     }
// }

// fn read_to_vec(file_name: &str) -> anyhow::Result<Vec<u8>> {
//     let mut buf = Vec::new();
//     std::fs::File::open(file_name)?.read_to_end(&mut buf)?;
//     Ok(buf)
// }

// fn main() -> anyhow::Result<()> {
//     let args = Args::parse();
//     parse_docx(&args.name)?;
//     Ok(())
// }

// // use docx_rust::document::BodyContent;
// // use docx_rust::document::Paragraph;
// // use docx_rust::DocxFile;
// // use std::any::Any;


// // fn main() {    
// //     let docx = DocxFile::from_file("docs/original.docx").unwrap();
// //     let mut docx = docx.parse().unwrap();

// //     /* 
// //     let para = Paragraph::default().push_text("Lorem Ipsum");
// //     docx.document.push(para);
    
// //     docx.write_file("docs/origin_appended.docx").unwrap();
// //     */

// //     let content = docx.document.body.content;
// //     for c in content.iter() {
// //         match c {
// //             BodyContent::Paragraph(p) => println!("para: {}", p.text()),
// //             _ => println!("something else")
// //         }
// //     }
// // }

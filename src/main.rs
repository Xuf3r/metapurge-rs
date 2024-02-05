mod consts;
mod xml_core;
mod utils;

use std::ffi::OsStr;
use std::fmt::Error;
use std::{fs, io};
use std::fs::File;
use std::io::{BufRead, Cursor, Read, SeekFrom, Write};
use std::ops::Deref;
use walkdir::{DirEntry, WalkDir};
use std::path::Path;
use std::str::from_utf8;
use std::time::Instant;
use crate::consts::*;
use regex::Regex;
use zip::write::FileOptions;
use zip::ZipWriter;

// fn edit_core_part_1(data: &mut CoreApp, ) {
//
// }

fn is_in(item: &DirEntry, filter: &Vec<&OsStr>) -> Result<Option<String>, std::io::Error> {

    let binding = item;
    let extension = binding.path().extension();
    if extension.is_none() {
        Ok(None)
    }
    else if filter.contains(&extension.unwrap()) {
        Ok(Some(item.path().to_str().unwrap().to_string()))
    }
    else {
        Ok(None)
    }

}

fn replace(data: &str) -> String {
    let re1 = Regex::new(r"<dc:title.*<dcterms:created").unwrap();
    let re2 = Regex::new(r"<cp:category.*</cp:coreProperties>").unwrap();
    re2.
        replace_all(&re1.replace_all(data, CoreXmlStr::TEXT_TEMPLATE_1), CoreXmlStr::TEXT_TEMPLATE_2)
        .to_string()
}

fn main() -> () {

    let start_time = Instant::now();


    let filter_vec = vec![OsStr::new("docx")];

    let (oks, errs): (Vec<_>, Vec<_>) = WalkDir::new("C:\\Users\\stp\\ferrprojs\\test0")
        .into_iter()
        .filter_map(Result::ok)
        .map(|path| is_in(&path, &filter_vec))
        .partition(Result::is_ok);

    let filtered: Vec<String> = oks.into_iter()
        .filter_map(|result| result.ok()) // Extract the `Ok` values
        .flatten() // Flatten the `Vec<Option<String>>` into a `Vec<String>`
        .collect();


    // println!("{:?}", &filtered);

    let file = fs::File::open(filtered.get(0).unwrap()).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();

    let outdocxpath = "example_temp.docx";
    let outfile = File::create(&outdocxpath).unwrap();
    let mut zipout = ZipWriter::new(outfile);


    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };
        let mut content = vec![];
        let mut replxml: String = String::new();
        let mut replxmlstr = "";
        if &outpath.clone().to_str().unwrap() == &"docProps/core.xml" {
            let num = file.by_ref().read_to_end(&mut content).expect("Unable to read file");
            let corexml = std::str::from_utf8(&content).unwrap();
            replxml = replace(corexml);
            replxmlstr = &replxml;
            // println!("core's contents are: {:?} with {:?} bytes", replxmlstr, replxml.len());
        }
        {
            let comment = file.comment();
            if !comment.is_empty() {
                // println!("File {i} comment: {comment}");
            }
        }

        if (*file.name()).ends_with('/') {
            // println!("File {} extracted to \"{}\"", i, outpath.display());
            zipout.add_directory(outpath.to_str().unwrap(), Default::default()).unwrap();
        }
        else if &outpath.clone().to_str().unwrap() == &"docProps/core.xml"{
            // let mut cursor = Cursor::new(replxmlstr);

            zipout.start_file(outpath.to_str().unwrap(), FileOptions::default()).unwrap();
            zipout.write_all(replxmlstr.as_bytes()).unwrap();
        }

        else {
            println!(
                // "File {} extracted to \"{}\" ({} bytes)",
                // i,
                // outpath.display(),
                // file.size()
            );
            // if let Some(p) = outpath.parent() {
            //     if !p.exists() {
            //         zipout.add_directory(p.to_str().unwrap(), Default::default()).unwrap();
            //     }
            // }
            // let mut outfile = fs::File::create(&outpath).unwrap();
            // io::copy(&mut file, &mut outfile).unwrap();
            file.read_to_end(&mut content).unwrap();
            zipout.start_file(outpath.to_str().unwrap(), FileOptions::default()).unwrap();
            zipout.write_all(content.as_slice()).unwrap();
        };

    }
    zipout.finish().unwrap();
    let elapsed_time = start_time.elapsed();

    // Print the elapsed time in seconds and milliseconds
    println!("Elapsed time: {} seconds and {} milliseconds",
             elapsed_time.as_secs(),
             elapsed_time.subsec_millis());

    fs::remove_file("example.docx").unwrap();
    fs::rename("example_temp.docx", "example.docx").unwrap();
}





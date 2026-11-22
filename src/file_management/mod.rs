use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use chrono::NaiveDate;

pub fn file_writer_from_naive_date(
    root_path: &std::path::Path,
    kind: &str,
    date: &NaiveDate,
) -> Result<std::io::BufWriter<std::fs::File>, std::io::Error> {
    let filename = date.format("%Y.jsonl").to_string();
    let path = root_path.join(kind).join(filename);
    if let Some(dir) = path.parent() {
        if !std::fs::exists(dir)? {
            std::fs::create_dir_all(dir)?;
        }
    }
    let file = OpenOptions::new().append(true).create(true).open(&path)?;
    let buf_writer = BufWriter::new(file);
    Ok(buf_writer)
}

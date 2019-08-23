use super::*;

use std::io::Write;
use std::collections::HashSet;

use tempfile::NamedTempFile;

const MPQ_FILE: &'static [u8] = include_bytes!("test.w3m");

fn create_temp_mpq() -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(MPQ_FILE).unwrap();
    file.flush().unwrap();

    file
}

#[test]
fn test_read_mpq_listfile() {
    let mpq_file = create_temp_mpq();
    let mpq_archive = MPQArchive::open(mpq_file.path().to_str().unwrap()).unwrap();
    let mpq_listfile = mpq_archive.open_file(&MPQPath::from_buf("(listfile)").unwrap()).unwrap();

    let contents = mpq_listfile.read_contents().unwrap();

    assert!(contents.len() > 0);
}

#[test]
fn test_get_all_files() {
    let mpq_file = create_temp_mpq();
    let mpq_archive = MPQArchive::open(mpq_file.path().to_str().unwrap()).unwrap();
    let all_files: HashSet<_> = mpq_archive.iter_files().unwrap().filter_map(|e| e).map(|e| e.as_cstr()).collect();

    assert!(all_files.contains("(listfile)"));
    assert!(all_files.contains("war3map.lua"));
    assert!(all_files.contains("war3map.w3c"));
}
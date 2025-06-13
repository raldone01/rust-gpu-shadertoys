use alloc::{borrow::ToOwned, string::String, vec::Vec};
use hashbrown::HashMap;

use crate::tar_gz_extract;

const MY_FILE: &[u8] = include_bytes!("test-archive/subfolder/my_file.txt");
const MY_FILE_PATH: &str = "subfolder/my_file.txt";
const LOREM_TXT: &[u8] = include_bytes!("test-archive/lorem.txt");
const LOREM_TXT_PATH: &str = "lorem.txt";
const TEST_FILE: &[u8] = include_bytes!("test-archive/test_file.txt");
const TEST_FILE_PATH: &str = "test_file.txt";

const USTAR_TAR: &[u8] = include_bytes!("test-ustar.tar");
const USTAR_TAR_GZ: &[u8] = include_bytes!("test-ustar.tar.gz");

fn assert_files(files: HashMap<String, Vec<u8>>) {
  let _dbg_file_paths: Vec<_> = files.keys().cloned().collect();
  let _dbg_file_contents: Vec<String> = files
    .values()
    .map(|v| String::from(String::from_utf8_lossy(v)))
    .collect();
  assert_eq!(files.len(), 3);
  assert_eq!(files.get(MY_FILE_PATH), Some(&MY_FILE.to_vec()));
  assert_eq!(files.get(LOREM_TXT_PATH), Some(&LOREM_TXT.to_vec()));
  assert_eq!(files.get(TEST_FILE_PATH), Some(&TEST_FILE.to_vec()));
}

#[test]
fn test_ustar_extract() {
  let files = tar_gz_extract::extract_tar_file(USTAR_TAR, 16 * 1024 * 1024)
    .expect("Failed to extract USTAR TAR");
  assert_files(files);
}

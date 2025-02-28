use std::path::{MAIN_SEPARATOR, MAIN_SEPARATOR_STR, Path};

pub struct FileUtil {
}

impl FileUtil {
    pub fn normalize_path(s: &str) -> String {
      let src_char = if MAIN_SEPARATOR == '/' {
        "\\" 
      } else { 
        "/" 
      };

      s.replace(src_char, MAIN_SEPARATOR_STR)
    }

    pub fn get_sub_path(path: &Path, base_dir: &str) -> String {
      let nbase_dir = FileUtil::normalize_path(base_dir);

      let base;
      if nbase_dir.ends_with(MAIN_SEPARATOR_STR) {
          base = nbase_dir;
      } else {
          base = nbase_dir + MAIN_SEPARATOR_STR;
      }

      let sub_path;

      let full_path = path.to_string_lossy();
      let nfull_path = FileUtil::normalize_path(&full_path);
      if nfull_path.starts_with(&base) {
          sub_path = &nfull_path[base.len()..];
      } else {
          sub_path = &nfull_path;
      }

      sub_path.to_owned()
  }
}

#[cfg(test)]
mod tests {
    use super::FileUtil;
    use std::path::Path;

    #[test]
    fn test_get_sub_path() {
        let p = Path::new("/some/where/on/the/rainbow.docx");
        let b = "/some/where/on/";
        assert_eq!(FileUtil::normalize_path("the/rainbow.docx"), FileUtil::get_sub_path(p, b));
    }

    #[test]
    fn test_get_sub_path1() {
        let p = Path::new("/some/where/on/the/rainbow.docx");
        let b = "/some/where/on";
        assert_eq!(FileUtil::normalize_path("the/rainbow.docx"), FileUtil::get_sub_path(p, b));
    }

    #[test]
    fn test_get_sub_path2() {
        let b = "/some/where/on/";
        let p = Path::new("/elsewhere/cloud.docx");
        assert_eq!(FileUtil::normalize_path("/elsewhere/cloud.docx"), FileUtil::get_sub_path(p, b));
    }
}
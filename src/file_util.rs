use std::path::Path;

pub struct FileUtil {
}

impl FileUtil {
    pub fn get_sub_path(path: &Path, base_dir: &str) -> String {
      let base;
      if base_dir.ends_with("/") {
          base = base_dir.to_owned();
      } else {
          base = base_dir.to_owned() + "/";
      }

      let sub_path;

      let full_path = path.to_string_lossy();
      if full_path.starts_with(&base) {
          sub_path = &full_path[base.len()..];
      } else {
          sub_path = &full_path;
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
        assert_eq!("the/rainbow.docx", FileUtil::get_sub_path(p, b));
    }

    #[test]
    fn test_get_sub_path1() {
        let p = Path::new("/some/where/on/the/rainbow.docx");
        let b = "/some/where/on";
        assert_eq!("the/rainbow.docx", FileUtil::get_sub_path(p, b));
    }

    #[test]
    fn test_get_sub_path2() {
        let b = "/some/where/on/";
        let p = Path::new("/elsewhere/cloud.docx");
        assert_eq!("/elsewhere/cloud.docx", FileUtil::get_sub_path(p, b));
    }
}
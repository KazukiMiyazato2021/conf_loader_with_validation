use std::io::Error;
use std::io::Read;
use std::fs::File;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}


fn load_file(file_path: &str) -> Result<String, Error> {
    let mut f = File::open(file_path)?;
    let mut s = String::new();
    let _ = f.read_to_string(&mut s);
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_read_file_and_convert_to_string() {
        // ファイルを読み込んで内容を確認
        let result = load_file("tests/case-1.conf");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "endpoint = localhost:3000\ndebug = true\nlog.file = /var/log/console.log\n");
    }
}

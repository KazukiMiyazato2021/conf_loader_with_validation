use std::io::Read;
use std::fs::File;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

fn load_file(file_path: &str) -> std::result::Result<std::string::String, std::io::Error> {
    let f = File::open(file_path);

    let mut f = match f {
        Ok(file) => file,
        Err(e) => return Err(e),
    };

    let mut s = String::new();

    match f.read_to_string(&mut s) {
        Ok(_) => Ok(s),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_read_file_and_convert_to_string() {
        let path = "sysctl.conf";
        std::fs::write(path, "endpoint = localhost:3000").expect("Failed to write test file");
        // ファイルを読み込んで内容を確認
        let result = load_file("sysctl.conf");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "endpoint = localhost:3000");
    
        // テスト用のファイルを削除
        std::fs::remove_file("sysctl.conf").expect("Failed to remove test file");
    }
}

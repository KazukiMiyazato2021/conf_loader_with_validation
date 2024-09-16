use std::io::Error;
use std::io::Read;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::collections::HashMap;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

enum ConfValue {
    String(String),
    Bool(bool),
    Number(i32),
    Conf,
}

type Conf = HashMap<String, ConfValue>;

pub fn parse(file_path: &str) -> Conf {
    if let Ok(lines) = read_lines(file_path) {
        // Consumes the iterator, returns an (Optional) String
        // イテレータを消費し、Option型のStringを返す。
        for line in lines.flatten() {
            println!("line {}", line);
            let key_value = parse_line(&line);
            if key_value.is_none() {
                continue;
            }
            println!("parse_line {:?}", key_value);
        }
    }
    let map: Conf = HashMap::new();
    map
}

type KeyValue<'a> = (&'a str, &'a str);
fn parse_line(line: &str) -> Option<KeyValue<'_>> {
    let l = line.trim();
    if l.is_empty() {
        return None;
    }
    if l.starts_with('#') || l.starts_with(';') {
        return None;
    }
    let vec = line.splitn(2, '=').collect::<Vec<&str>>();
    if vec.len() != 2 {
        return None;
    }
    let key = vec[0].trim();
    let value = vec[1].trim();
    if key.is_empty() || value.is_empty() {
        return None;
    }
    Some((key, value))
}


/*
1. パスを受け取る（or ファイルインスタンスを受け取る）
2. ファイルを読み込む
3. 1行ずつパースしてHashMapにいれる
    # か ; で始まる行はコメント
4. 
 */
fn load_file(file_path: &str) -> Result<String, Error> {
    let mut f = File::options().read(true).open(file_path)?;
    let mut s = String::new();
    let _ = f.read_to_string(&mut s);
    Ok(s)
}

fn read_lines<P>(file_path: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::options().read(true).open(file_path)?;
    Ok(io::BufReader::new(file).lines())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_read_file_and_convert_to_string() {
        // ファイルを読み込んで内容を確認
        let _ = parse("tests/case-1.conf");
        // assert!(result.is_ok());
        // assert_eq!(result.unwrap(), "endpoint = localhost:3000\ndebug = true\nlog.file = /var/log/console.log\n");
    }
}

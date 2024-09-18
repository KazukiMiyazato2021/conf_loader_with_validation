use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::collections::HashMap;
use regex::Regex;

type Conf = HashMap<String, ConfValue>;
#[derive(Debug)]
#[derive(PartialEq)]
enum ConfValue {
    String(String),
    Conf(Conf),
}

pub fn parse(file_path: &str) -> Conf {
    let mut map: Conf = HashMap::new();
    if let Ok(lines) = read_lines(file_path) {
        // Consumes the iterator, returns an (Optional) String
        // イテレータを消費し、Option型のStringを返す。
        for line in lines.flatten() {
            let key_value = parse_line(&line);
            if key_value.is_none() {
                continue;
            }
            let (key, value): (&str, &str) = key_value.unwrap();
            add_value(&mut map, key, value);
        }
    }
    map
}

fn parse_schema(file_path: &str) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();
    if let Ok(lines) = read_lines(file_path) {
        for line in lines.flatten() {
            let key_value = parse_schema_line(&line);
            if key_value.is_none() {
                continue;
            }
            let (key, value): (&str, &str) = key_value.unwrap();
            map.insert(key.to_string(), value.to_string());
        }
    }
    map
}

fn add_value(map: &mut Conf, key: &str, value: &str) {
    let binding = key.splitn(2, '.').collect::<Vec<&str>>();
    let keys = binding.as_slice();
    if keys.len() == 1 {
        map.insert(key.to_string(), ConfValue::String(value.to_string()));
        return;
    }
    // キーがネストしているとき
    if map.contains_key(keys[0]) {
        let conf_value: &mut ConfValue = map.get_mut(keys[0]).unwrap();
        match conf_value {
            ConfValue::String(ref _s) => {
                let mut child_map: Conf = HashMap::new();
                add_value(&mut child_map, keys[1], value);
                map.insert(keys[0].to_string(), ConfValue::Conf(child_map));
            },
            // すでにある値がMapだった場合
            ConfValue::Conf(ref mut child_map ) => {
                add_value(child_map, keys[1], value);
            },
        }
    } else {
        let mut child_map: Conf = HashMap::new();
        add_value(&mut child_map, keys[1], value);
        map.insert(keys[0].to_string(), ConfValue::Conf(child_map));
    }
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

fn split_by_str<'a>(s: &'a str, delim: &str) -> Option<std::vec::Vec<&'a str>> {
    if !s.contains(delim) {
        return None;
    }
    let re = Regex::new(delim).unwrap();
    let parts: Vec<&str> = re.splitn(s, 2).collect();
    Some(parts)
}

fn parse_schema_line(line: &str) -> Option<KeyValue<'_>> {
    let l = line.trim();
    if l.is_empty() {
        return None;
    }
    let vec = split_by_str(l, "->")?;
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

fn read_lines<P>(file_path: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::options().read(true).open(file_path)?;
    Ok(io::BufReader::new(file).lines())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_read_file() {
        // ファイルを読み込んで内容を確認
        assert_eq!(parse("tests/case-1.conf"), HashMap::<String, ConfValue>::from([
            ("endpoint".to_string(), ConfValue::String("localhost:3000".to_string())),
            ("log".to_string(), ConfValue::Conf(HashMap::from([
                ("file".to_string(), ConfValue::String("/var/log/console.log".to_string()),)
            ]))),
            ("debug".to_string(), ConfValue::String("true".to_string())),
        ]));

        assert_eq!(parse("tests/case-2.conf"), HashMap::<String, ConfValue>::from([
            ("endpoint".to_string(), ConfValue::String("localhost:3000".to_string())),
            ("log".to_string(), ConfValue::Conf(HashMap::from([
                ("file".to_string(), ConfValue::String("/var/log/console.log".to_string()),),
                ("name".to_string(), ConfValue::String("default.log".to_string())),
            ]))),
        ]));
    }
    #[test]
    fn can_read_schema() {
        assert_eq!(parse_schema_line("log.file -> string"), Some(("log.file", "string")));
        assert_eq!(parse_schema("tests/data.schema"), HashMap::<String, String>::from([
            ("endpoint".to_string(), "string".to_string()),
            ("debug".to_string(), "bool".to_string()),
            ("log.file".to_string(), "string".to_string()),
        ]))
    }
}

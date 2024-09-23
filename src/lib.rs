use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::collections::HashMap;
use std::str::FromStr;
use regex::Regex;

trait Value: std::fmt::Debug {}

type Conf = HashMap<String, Box<dyn Value>>;
#[derive(Debug)]
enum ConfValue {
    StrValue(String),
    BoolValue(bool),
    NumberValue(f64),
    Conf(Conf),
}

impl Value for ConfValue {}

enum Type {
    StringType,
    BoolType,
    NumberType,
}

impl FromStr for Type {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "string" => Ok(Type::StringType),
            "bool" => Ok(Type::BoolType),
            "number" => Ok(Type::NumberType),
            _ => Err(format!("Invalid type: {}", s)),
        }
    }
}

pub fn parse(file_path: &str, schema_path: Option<&str>) -> Conf {
    let schema: HashMap<String, Type> = match schema_path {
        Some(path) => parse_schema(path),
        None => HashMap::new(),
    };
    parse_conf(file_path, schema)
}

fn parse_conf(file_path: &str, schema: HashMap<String, Type>) -> Conf {
    let mut map: Conf = HashMap::new();
    if let Ok(lines) = read_lines(file_path) {
        for line in lines.flatten() {
            let key_value = parse_line(&line);
            if key_value.is_none() {
                continue;
            }
            let (key, value): (&str, &str) = key_value.unwrap();
            let typed_value = match schema.contains_key(key) {
                true => validate(value, schema.get(key).unwrap()).unwrap(),
                false => Box::new(ConfValue::StrValue(value.to_string())),
            };
            add_value(&mut map, key, typed_value);
        }
    }
    map
}

fn validate(s: &str, t: &Type) -> Result<Box<dyn Value>, String> {
    match t {
        Type::StringType => Ok(Box::new(ConfValue::StrValue(s.to_string()))),
        Type::BoolType => match s {
            "true" => Ok(Box::new(ConfValue::BoolValue(true))),
            "false" => Ok(Box::new(ConfValue::BoolValue(false))),
            _ => Err("Invalid boolean value".to_string()),
        },
        Type::NumberType => {
            if let Ok(number) = f64::from_str(s) {
                Ok(Box::new(ConfValue::NumberValue(number)))
            } else {
                Err("Invalid number value".to_string())
            }
        }
    }
}

fn parse_schema(file_path: &str) -> HashMap<String, Type> {
    let mut map: HashMap<String, Type> = HashMap::new();
    if let Ok(lines) = read_lines(file_path) {
        for line in lines.flatten() {
            let key_value = parse_schema_line(&line);
            if key_value.is_none() {
                continue;
            }
            let (key, t): (&str, &str) = key_value.unwrap();
            let type_enum = t.parse::<Type>()?;
            map.insert(key.to_string(), type_enum);
        }
    }
    map
}

fn add_value(map: &mut Conf, key: &str, value: Box<dyn Value>) {
    let binding = key.splitn(2, '.').collect::<Vec<&str>>();
    let keys = binding.as_slice();
    if keys.len() == 1 {
        map.insert(key.to_string(), value);
        return;
    }
    // キーがネストしているとき
    if map.contains_key(keys[0]) {
        let conf_value: &mut Box<dyn Value> = map.get_mut(keys[0]).unwrap();
        match conf_value.as_mut() {
            ConfValue::StrValue(_) => {
                let mut child_map: Conf = HashMap::new();
                add_value(&mut child_map, keys[1], value);
                map.insert(keys[0].to_string(), Box::new(ConfValue::Conf(child_map)));
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
            ("endpoint".to_string(), ConfValue::StrValue("localhost:3000".to_string())),
            ("log".to_string(), ConfValue::Conf(HashMap::from([
                ("file".to_string(), ConfValue::StrValue("/var/log/console.log".to_string()),)
            ]))),
            ("debug".to_string(), ConfValue::StrValue("true".to_string())),
        ]));

        assert_eq!(parse("tests/case-2.conf"), HashMap::<String, ConfValue>::from([
            ("endpoint".to_string(), ConfValue::StrValue("localhost:3000".to_string())),
            ("log".to_string(), ConfValue::Conf(HashMap::from([
                ("file".to_string(), ConfValue::StrValue("/var/log/console.log".to_string()),),
                ("name".to_string(), ConfValue::StrValue("default.log".to_string())),
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

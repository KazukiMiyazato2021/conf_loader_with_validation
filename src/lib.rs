use std::fmt::Debug;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::collections::HashMap;
use std::str::FromStr;
use regex::Regex;
use std::error::Error;

trait Value<T>: Debug {
    fn as_mut(&mut self) -> &mut T;
}

type Conf = HashMap<String, Box<dyn Value<ConfValue>>>;
#[derive(Debug)]
enum ConfValue {
    StrValue(String),
    BoolValue(bool),
    NumberValue(f64),
    Conf(Conf),
}

impl PartialEq for ConfValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ConfValue::StrValue(a), ConfValue::StrValue(b)) => a == b,
            (ConfValue::BoolValue(a), ConfValue::BoolValue(b)) => a == b,
            (ConfValue::NumberValue(a), ConfValue::NumberValue(b)) => a == b,
            (ConfValue::Conf(a), ConfValue::Conf(b)) => {
                // Compare the HashMaps manually
                if a.len() != b.len() {
                    return false;
                }
                for (key, val_a) in a {
                    if let Some(val_b) = b.get(key) {
                        if !val_a.as_ref().eq(val_b.as_ref()) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            },
            _ => false,
        }
    }
}

impl Value<ConfValue> for ConfValue {
    fn as_mut(&mut self) -> &mut ConfValue {
        self
    }
}

#[derive(Debug, PartialEq)]
enum SchemaType {
    String,
    Bool,
    Number,
}

impl FromStr for SchemaType {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "string" => Ok(SchemaType::String),
            "bool" => Ok(SchemaType::Bool),
            "number" => Ok(SchemaType::Number),
            _ => Err(format!("Invalid type: {}", s)),
        }
    }
}

pub fn parse(file_path: &str, schema_path: Option<&str>) -> Result<Conf, Box<dyn Error>> {
    let schema: HashMap<String, SchemaType> = match schema_path {
        Some(path) => parse_schema(path)?,
        None => HashMap::new(),
    };
    Ok(parse_conf(file_path, schema))
}

fn parse_conf(file_path: &str, schema: HashMap<String, SchemaType>) -> Conf {
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

fn validate(s: &str, t: &SchemaType) -> Result<Box<dyn Value<ConfValue>>, String> {
    match t {
        SchemaType::String => Ok(Box::new(ConfValue::StrValue(s.to_string()))),
        SchemaType::Bool => match s {
            "true" => Ok(Box::new(ConfValue::BoolValue(true))),
            "false" => Ok(Box::new(ConfValue::BoolValue(false))),
            _ => Err("Invalid boolean value".to_string()),
        },
        SchemaType::Number => {
            if let Ok(number) = f64::from_str(s) {
                Ok(Box::new(ConfValue::NumberValue(number)))
            } else {
                Err("Invalid number value".to_string())
            }
        }
    }
}

fn parse_schema(file_path: &str) -> Result<HashMap<String, SchemaType>, Box<dyn Error>> {
    let mut map: HashMap<String, SchemaType> = HashMap::new();
    if let Ok(lines) = read_lines(file_path) {
        for line in lines.flatten() {
            let key_value = parse_schema_line(&line);
            if key_value.is_none() {
                continue;
            }
            let (key, t): (&str, &str) = key_value.unwrap();
            let type_enum = t.parse::<SchemaType>()?;
            map.insert(key.to_string(), type_enum);
        }
    }
    Ok(map)
}

fn add_value(map: &mut Conf, key: &str, value: Box<dyn Value<ConfValue>>) {
    let binding = key.splitn(2, '.').collect::<Vec<&str>>();
    let keys = binding.as_slice();
    if keys.len() == 1 {
        map.insert(key.to_string(), value);
        return;
    }
    // キーがネストしているとき
    if map.contains_key(keys[0]) {
        let conf_value: &mut Box<dyn Value<ConfValue>> = map.get_mut(keys[0]).unwrap();
        match conf_value.as_mut().as_mut() {
            // すでにある値がMapだった場合
            ConfValue::Conf(child_map ) => {
                add_value(child_map, keys[1], value);
            },
            _ => {
                let mut child_map: Conf = HashMap::new();
                add_value(&mut child_map, keys[1], value);
                map.insert(keys[0].to_string(), Box::new(ConfValue::Conf(child_map)));
            },
        }
    } else {
        let mut child_map: Conf = HashMap::new();
        add_value(&mut child_map, keys[1], value);
        map.insert(keys[0].to_string(), Box::new(ConfValue::Conf(child_map)));
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
        let result1 = parse("tests/case-1.conf", Some("tests/data.schema"));
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), HashMap::<String, Box<dyn Value<ConfValue>>>::from([
            ("endpoint".to_string(), Box::new(ConfValue::StrValue("localhost:3000".to_string()))),
            ("log".to_string(), Box::new(ConfValue::Conf(HashMap::<String, Box<dyn Value<ConfValue>>>::from([
                ("file".to_string(), Box::new(ConfValue::StrValue("/var/log/console.log".to_string())))
            ])))),
            ("debug".to_string(), Box::new(ConfValue::StrValue("true".to_string()))),
        ]));

        // assert_eq!(parse("tests/case-2.conf", None), HashMap::<String, ConfValue>::from([
        //     ("endpoint".to_string(), ConfValue::StrValue("localhost:3000".to_string())),
        //     ("log".to_string(), ConfValue::Conf(HashMap::from([
        //         ("file".to_string(), ConfValue::StrValue("/var/log/console.log".to_string()),),
        //         ("name".to_string(), ConfValue::StrValue("default.log".to_string())),
        //     ]))),
        // ]));
    }
    #[test]
    fn can_read_schema() {
        assert_eq!(parse_schema_line("log.file -> string"), Some(("log.file", "string")));
        let result = parse_schema("tests/data.schema");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), HashMap::<String, SchemaType>::from([
            ("endpoint".to_string(), SchemaType::String),
            ("debug".to_string(), SchemaType::Bool),
            ("log.file".to_string(), SchemaType::String),
        ]));
    }
}

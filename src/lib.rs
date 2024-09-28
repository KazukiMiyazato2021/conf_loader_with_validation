use std::fmt::Debug;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::collections::BTreeMap;
use std::str::FromStr;
use regex::Regex;
use std::error::Error;
use std::ptr;
use std::any::Any;

trait Value<T>: Debug {
    fn as_any(&self) -> &dyn Any;
    fn as_mut(&mut self) -> &mut T;
}

type Conf = BTreeMap<String, Box<dyn Value<ConfValue>>>;
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
                let ptr_a = a as *const Conf;
                let ptr_b = b as *const Conf;
                ptr::eq(ptr_a, ptr_b)
            },
            _ => false,
        }
    }
}

impl Value<ConfValue> for ConfValue {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_mut(&mut self) -> &mut ConfValue {
        self
    }
}

impl PartialEq for dyn Value<ConfValue> {
    fn eq(&self, other: &Self) -> bool {
        if let (Some(self_val), Some(other_val)) = (self.as_any().downcast_ref::<ConfValue>(), other.as_any().downcast_ref::<ConfValue>()) {
            self_val == other_val
        } else {
            false
        }
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
    let schema: BTreeMap<String, SchemaType> = match schema_path {
        Some(path) => parse_schema(path)?,
        None => BTreeMap::new(),
    };
    Ok(parse_conf(file_path, schema))
}

fn parse_conf(file_path: &str, schema: BTreeMap<String, SchemaType>) -> Conf {
    let mut map: Conf = BTreeMap::new();
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

fn parse_schema(file_path: &str) -> Result<BTreeMap<String, SchemaType>, Box<dyn Error>> {
    let mut map: BTreeMap<String, SchemaType> = BTreeMap::new();
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
                let mut child_map: Conf = BTreeMap::new();
                add_value(&mut child_map, keys[1], value);
                map.insert(keys[0].to_string(), Box::new(ConfValue::Conf(child_map)));
            },
        }
    } else {
        let mut child_map: Conf = BTreeMap::new();
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
        assert_eq!(result1.unwrap(), BTreeMap::<String, Box<dyn Value<ConfValue>>>::from([
            ("endpoint".to_string(), Box::new(ConfValue::StrValue("localhost:3000".to_string())) as Box<dyn Value<ConfValue>>),
            ("debug".to_string(), Box::new(ConfValue::BoolValue(true)) as Box<dyn Value<ConfValue>>),
            ("log".to_string(), Box::new(ConfValue::Conf(BTreeMap::<String, Box<dyn Value<ConfValue>>>::from([
                ("file".to_string(), Box::new(ConfValue::StrValue("/var/log/console.log".to_string())) as Box<dyn Value<ConfValue>>)
            ]))) as Box<dyn Value<ConfValue>>),
        ]));

        // assert_eq!(parse("tests/case-2.conf", None), BTreeMap::<String, ConfValue>::from([
        //     ("endpoint".to_string(), ConfValue::StrValue("localhost:3000".to_string())),
        //     ("log".to_string(), ConfValue::Conf(BTreeMap::from([
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
        assert_eq!(result.unwrap(), BTreeMap::<String, SchemaType>::from([
            ("endpoint".to_string(), SchemaType::String),
            ("debug".to_string(), SchemaType::Bool),
            ("log.file".to_string(), SchemaType::String),
        ]));
    }
}

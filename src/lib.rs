use std::fmt;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::collections::HashMap;
use std::str::FromStr;
use regex::Regex;
use std::error::Error;

// エラー型を定義
#[derive(Debug)]
struct TypeMismatchError;

impl fmt::Display for TypeMismatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Type mismatch error")
    }
}

impl Error for TypeMismatchError {}

// ConfValue 型
#[derive(Debug)]
enum ConfValue {
    StrValue(String),
    BoolValue(bool),
    NumberValue(f64),
    Conf(Box<ConfList>), // Linked List 形式に変更
}

// ConfValue に型指定アクセス用メソッドを追加
impl ConfValue {
    fn as_str(&self) -> Result<&String, TypeMismatchError> {
        if let ConfValue::StrValue(ref value) = self {
            Ok(value)
        } else {
            Err(TypeMismatchError)
        }
    }

    fn as_bool(&self) -> Result<&bool, TypeMismatchError> {
        if let ConfValue::BoolValue(value) = self {
            Ok(value)
        } else {
            Err(TypeMismatchError)
        }
    }

    fn as_number(&self) -> Result<&f64, TypeMismatchError> {
        if let ConfValue::NumberValue(value) = self {
            Ok(value)
        } else {
            Err(TypeMismatchError)
        }
    }

    fn as_conf(&self) -> Result<&Box<ConfList>, TypeMismatchError> {
        if let ConfValue::Conf(ref conf) = self {
            Ok(conf)
        } else {
            Err(TypeMismatchError)
        }
    }
}

// ノードを表す構造体
#[derive(Debug)]
struct Node {
    key: String,
    value: ConfValue,
    next: Option<Box<Node>>,
}

// Linked List 形式の構造体
#[derive(Debug)]
struct ConfList {
    head: Option<Box<Node>>,
}

impl ConfList {
    fn new() -> Self {
        ConfList { head: None }
    }

    // 要素を追加する insert() メソッド
    fn insert(&mut self, key: String, value: ConfValue) {
        let new_node = Box::new(Node {
            key,
            value,
            next: self.head.take(),
        });
        self.head = Some(new_node);
    }

    fn add_value(&mut self, key: &str, value: ConfValue) {
        let binding: Vec<&str> = key.splitn(2, '.').collect::<Vec<&str>>();
        let keys: &[&str] = binding.as_slice();
        // ネストしてないキー
        if keys.len() == 1 {
            self.insert(key.to_string(), value);
            return;
        }
        // キーがネストしているとき
        if self.contains_key(keys[0]) {
            let conf_value = self.get(keys[0]).unwrap();
            match conf_value {
                // すでにある値がMapだった場合
                ConfValue::Conf(child_map) => {
                    child_map.add_value(keys[1], value);
                },
                // ↓ Map 以外はすべて同じ処理
                ConfValue::StrValue(_) => {
                    let mut child_map = Box::new(ConfList::new());
                    child_map.add_value(keys[1], value);
                    self.insert(keys[0].to_string(), ConfValue::Conf(child_map));
                },
                ConfValue::BoolValue(_) => {
                    let mut child_map = Box::new(ConfList::new());
                    child_map.add_value(keys[1], value);
                    self.insert(keys[0].to_string(), ConfValue::Conf(child_map));
                },
                ConfValue::NumberValue(_) => {
                    let mut child_map = Box::new(ConfList::new());
                    child_map.add_value(keys[1], value);
                    self.insert(keys[0].to_string(), ConfValue::Conf(child_map));
                },
            }
        } else {
            let mut child_map = Box::new(ConfList::new());
            child_map.add_value(keys[1], value);
            self.insert(keys[0].to_string(), ConfValue::Conf(child_map));
        }
    }

    // 要素が含まれているか確認する contains_key() メソッド
    fn contains_key(&self, key: &str) -> bool {
        let mut current = &self.head;
        while let Some(ref node) = current {
            if node.key == key {
                return true;
            }
            current = &node.next;
        }
        false
    }

    // キーを使って値を取得する get() メソッド
    fn get(&self, key: &str) -> Option<&ConfValue> {
        let mut current = &self.head;
        while let Some(ref node) = current {
            if node.key == key {
                return Some(&node.value);
            }
            current = &node.next;
        }
        None
    }
}

#[derive(Debug, PartialEq)]
enum SchemaType {
    String,
    Bool,
    Number,
}

trait Value<T>: std::fmt::Debug {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_mut(&mut self) -> &mut T;
}
impl Value<ConfValue> for ConfValue {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_mut(&mut self) -> &mut ConfValue {
        self
    }
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

pub fn parse(file_path: &str, schema_path: Option<&str>) -> Result<ConfList, Box<dyn Error>> {
    let schema: HashMap<String, SchemaType> = match schema_path {
        Some(path) => parse_schema(path)?,
        None => HashMap::new(),
    };
    Ok(parse_conf(file_path, schema))
}

fn parse_conf(file_path: &str, schema: HashMap<String, SchemaType>) -> ConfList {
    let mut map = ConfList::new();
    if let Ok(lines) = read_lines(file_path) {
        for line in lines.flatten() {
            let key_value = parse_line(&line);
            if key_value.is_none() {
                continue;
            }
            let (key, value): (&str, &str) = key_value.unwrap();
            let typed_value = match schema.contains_key(key) {
                true => validate(value, schema.get(key).unwrap()).unwrap(),
                false => ConfValue::StrValue(value.to_string()),
            };
            map.add_value(key, typed_value);
        }
    }
    map
}

fn validate(s: &str, t: &SchemaType) -> Result<ConfValue, String> {
    match t {
        SchemaType::String => Ok(ConfValue::StrValue(s.to_string())),
        SchemaType::Bool => match s {
            "true" => Ok(ConfValue::BoolValue(true)),
            "false" => Ok(ConfValue::BoolValue(false)),
            _ => Err("Invalid boolean value".to_string()),
        },
        SchemaType::Number => {
            if let Ok(number) = f64::from_str(s) {
                Ok(ConfValue::NumberValue(number))
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
            ("endpoint".to_string(), Box::new(ConfValue::StrValue("localhost:3000".to_string())) as Box<dyn Value<ConfValue>>),
            ("debug".to_string(), Box::new(ConfValue::BoolValue(true)) as Box<dyn Value<ConfValue>>),
            ("log".to_string(), Box::new(ConfValue::Conf(HashMap::<String, Box<dyn Value<ConfValue>>>::from([
                ("file".to_string(), Box::new(ConfValue::StrValue("/var/log/console.log".to_string())) as Box<dyn Value<ConfValue>>)
            ]))) as Box<dyn Value<ConfValue>>),
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

use std::cell::{RefCell, RefMut};
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::collections::HashMap;
use std::rc::Rc;
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

// テスト用
type ConfVec = Vec<(String, ConfVecValue)>;
#[derive(Debug, PartialEq)]
enum ConfVecValue {
    StrValue(String),
    BoolValue(bool),
    NumberValue(f64),
    Conf(ConfVec),
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

    fn as_bool(&self) -> Result<bool, TypeMismatchError> {
        if let ConfValue::BoolValue(value) = self {
            Ok(*value)
        } else {
            Err(TypeMismatchError)
        }
    }

    fn as_number(&self) -> Result<f64, TypeMismatchError> {
        if let ConfValue::NumberValue(value) = self {
            Ok(*value)
        } else {
            Err(TypeMismatchError)
        }
    }

    fn as_conf(&self) -> Result<&ConfList, TypeMismatchError> {
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
    value: RefCell<ConfValue>, // RefCell で内部を可変にする
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

    // 要素が含まれているか確認する contains_key() メソッド
    fn contains_key(&self, key: &str) -> bool {
        let mut current = &self.head;
        while let Some(node) = current {
            if node.key == key {
                return true;
            }
            current = &node.next;
        }
        false
    }

    fn get(&mut self, key: &str) -> Option<RefMut<'_, ConfValue>> {
        let mut current = &self.head;
        while let Some(node) = current {
            let value = (&(*node).value).borrow_mut();
            if node.key == key {
                return Some(value);
            }
            current = &node.next;
        }
        None
    }

    // 要素を追加する insert() メソッド
    fn insert(&mut self, key: String, value: ConfValue) {
        let new_node = Box::new(Node {
            key,
            value: RefCell::new(value),  // RefCell で包む
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
            let mut conf_value: RefMut<'_, ConfValue> = self.get(keys[0]).unwrap();
            let new_value: ConfValue = match &mut *conf_value {
                // すでにある値がNodeだった場合
                ConfValue::Conf(child_node) => {
                    child_node.add_value(keys[1], value);
                    return;
                },
                // ↓ Node 以外はすべて同じ処理
                ConfValue::StrValue(_) => {
                    let mut child_node = Box::new(ConfList::new());
                    child_node.add_value(keys[1], value);
                    ConfValue::Conf(child_node)
                },
                ConfValue::BoolValue(_) => {
                    let mut child_node = Box::new(ConfList::new());
                    child_node.add_value(keys[1], value);
                    ConfValue::Conf(child_node)
                },
                ConfValue::NumberValue(_) => {
                    let mut child_node = Box::new(ConfList::new());
                    child_node.add_value(keys[1], value);
                    ConfValue::Conf(child_node)
                },
            };
            drop(conf_value);  // 明示的に借用を解除
            self.insert(keys[0].to_string(), new_value);
        } else {
            let mut child_node = Box::new(ConfList::new());
            child_node.add_value(keys[1], value);
            self.insert(keys[0].to_string(), ConfValue::Conf(child_node));
        }
    }

    // テスト用 vecに変換する
    fn to_vec(&self) -> ConfVec {
        let mut vec: ConfVec = Vec::new();
        let mut current = self.head.as_ref();

        while let Some(node) = current {
            let v = node.value.borrow_mut();
            let new_value: ConfVecValue = match &*v {
                ConfValue::Conf(child_node) => {
                    ConfVecValue::Conf(child_node.to_vec())
                },
                ConfValue::StrValue(v) => {
                    ConfVecValue::StrValue(v.clone())
                },
                ConfValue::BoolValue(v) => {
                    ConfVecValue::BoolValue(*v)
                },
                ConfValue::NumberValue(v) => {
                    ConfVecValue::NumberValue(*v)
                },
            };
            vec.insert(0, (node.key.clone(), new_value));
            current = node.next.as_ref();
        }

        vec
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
    fn can_parse_conf_with_schema() {
        // ファイルを読み込んで内容を確認
        let result: Result<ConfList, Box<dyn Error>> = parse("tests/case-1.conf", Some("tests/data.schema"));
        assert!(result.is_ok());
        let mut conf = result.unwrap();
        assert_eq!(conf.to_vec(), vec![
            ("endpoint".to_string(), ConfVecValue::StrValue("localhost:3000".to_string())),
            ("debug".to_string(), ConfVecValue::BoolValue(true)),
            ("log".to_string(), ConfVecValue::Conf(vec![
                ("file".to_string(), ConfVecValue::StrValue("/var/log/console.log".to_string()))
            ])),
        ]);
        assert!(conf.contains_key("endpoint"));
        assert!(conf.contains_key("log"));
        assert_eq!(conf.get("endpoint").unwrap().as_str().unwrap(), &"localhost:3000".to_string());
        assert!(conf.get("endpoint").unwrap().as_bool().is_err());
        assert!(conf.get("debug").unwrap().as_bool().unwrap());
    }
    #[test]
    fn can_parse_conf_without_schema() {
        // ファイルを読み込んで内容を確認
        let result: Result<ConfList, Box<dyn Error>> = parse("tests/case-1.conf", None);
        assert!(result.is_ok());
        assert_eq!(result.as_ref().unwrap().to_vec(), vec![
            ("endpoint".to_string(), ConfVecValue::StrValue("localhost:3000".to_string())),
            ("debug".to_string(), ConfVecValue::StrValue("true".to_string())),
            ("log".to_string(), ConfVecValue::Conf(vec![
                ("file".to_string(), ConfVecValue::StrValue("/var/log/console.log".to_string()))
            ])),
        ]);
    }
}

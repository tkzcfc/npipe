extern crate prost_build;
use heck::{ToSnakeCase, ToUpperCamelCase};
use prost_build::Config;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::env::set_var;
use std::io;
use std::path::Path;
use std::{env, fs};

#[derive(Debug)]
struct MessageInfo {
    name: String,
    package: String,
    id: u32,
}

const ANNOTATION_PREFIX: &'static str = "//@build_automatically_generate_message_id@";

// https://docs.rs/prost-build/latest/prost_build/
fn main() -> io::Result<()> {
    // github访问太慢了,windows直接用下载好的
    if cfg!(windows) {
        set_var("PROTOC", "bin/protoc.exe");
    } else {
        return Ok(());
        // let (protoc_bin, _) = init("22.0").unwrap();
        // println!("protoc_bin: {}", protoc_bin.to_str().unwrap());
        // set_var("PROTOC", protoc_bin);
    }

    // 需要导出的协议文件列表
    let proto_file_list = [
        "src/pb/Client_Server.proto",
        "src/pb/Server_Client.proto",
        "src/pb/Generic.proto",
    ];
    let include_list = ["src/pb"];
    let out_dir = Path::new("src");

    let mut backups = HashMap::new();

    // 备份协议文件
    for proto_file in &proto_file_list {
        let contents = fs::read_to_string(proto_file)?;
        let proto_file = env::current_dir()?.join(proto_file);
        backups.insert(proto_file, contents);
    }

    let result = build(&proto_file_list, &include_list, &out_dir);

    // 还原协议文件
    for proto_file in &proto_file_list {
        let proto_file = env::current_dir()?.join(proto_file);
        let content = backups.get(&proto_file).unwrap();

        let lines = content
            .lines()
            .map(|line| {
                if line.starts_with(ANNOTATION_PREFIX) {
                    return line.trim_start_matches(ANNOTATION_PREFIX);
                }
                line
            })
            .collect::<Vec<&str>>();

        fs::write(&proto_file, lines.join("\n"))?;
    }

    match result {
        Ok(messages) => {
            // 检查消息id是否重复
            let mut set = HashSet::new();
            for it in messages.iter() {
                if !set.insert(it.id) {
                    panic!("消息id重复：{:?}", it);
                }
            }
        }
        Err(err) => {
            return Err(err);
        }
    }

    Ok(())
}

/// Converts a `camelCase` or `SCREAMING_SNAKE_CASE` identifier to a `lower_snake` case Rust field
/// identifier.
fn to_snake(s: &str) -> String {
    let mut ident = s.to_snake_case();

    // Use a raw identifier if the identifier matches a Rust keyword:
    // https://doc.rust-lang.org/reference/keywords.html.
    match ident.as_str() {
        // 2015 strict keywords.
        | "as" | "break" | "const" | "continue" | "else" | "enum" | "false"
        | "fn" | "for" | "if" | "impl" | "in" | "let" | "loop" | "match" | "mod" | "move" | "mut"
        | "pub" | "ref" | "return" | "static" | "struct" | "trait" | "true"
        | "type" | "unsafe" | "use" | "where" | "while"
        // 2018 strict keywords.
        | "dyn"
        // 2015 reserved keywords.
        | "abstract" | "become" | "box" | "do" | "final" | "macro" | "override" | "priv" | "typeof"
        | "unsized" | "virtual" | "yield"
        // 2018 reserved keywords.
        | "async" | "await" | "try" => ident.insert_str(0, "r#"),
        // the following keywords are not supported as raw identifiers and are therefore suffixed with an underscore.
        "self" | "super" | "extern" | "crate" => ident += "_",
        _ => (),
    }
    ident
}

/// Converts a `snake_case` identifier to an `UpperCamel` case Rust type identifier.
fn to_upper_camel(s: &str) -> String {
    let mut ident = s.to_upper_camel_case();

    // Suffix an underscore for the `Self` Rust keyword as it is not allowed as raw identifier.
    if ident == "Self" {
        ident += "_";
    }
    ident
}

fn format_package_name(message_info: &MessageInfo) -> String {
    let package = message_info.package.replace("PB.", "");
    to_snake(&package)
}

fn format_message_full_type(message_info: &MessageInfo) -> String {
    format!(
        "super::{}::{}",
        format_package_name(message_info),
        to_upper_camel(&message_info.name)
    )
}

fn format_message_type_name(message_info: &MessageInfo) -> String {
    format!(
        "{}{}",
        to_upper_camel(&format_package_name(message_info)),
        to_upper_camel(&message_info.name)
    )
}

fn build_code(messages: &Vec<MessageInfo>) -> String {
    let mut code_message = Vec::new();
    let mut code_get_message_id = Vec::new();
    let mut code_decode_message = Vec::new();
    let mut code_encode_message = Vec::new();
    let mut code_get_message_size = Vec::new();
    let mut code_encode_raw_message = Vec::new();
    let mut code_serialize_to_json = Vec::new();

    messages.iter().for_each(|info| {
        let code = format!(
            "    {}({}),",
            format_message_type_name(info),
            format_message_full_type(info)
        );
        code_message.push(code);

        let code = format!(
            "        MessageType::{}(_) => Some({}u32),",
            format_message_type_name(info),
            info.id
        );
        code_get_message_id.push(code);

        let code = format!(
            r#"        {}u32 => match {}::decode(bytes) {{
            Ok(message) => Ok(MessageType::{}(message)),
            Err(err) => Err(err),
        }},"#,
            info.id,
            format_message_full_type(info),
            format_message_type_name(info)
        );
        code_decode_message.push(code);

        let code = format!(
            r#"        MessageType::{}(msg) => Some(({}u32, msg.encode_to_vec())),"#,
            format_message_type_name(info),
            info.id
        );
        code_encode_message.push(code);

        let code = format!(
            r#"        MessageType::{}(msg) => msg.encoded_len(),"#,
            format_message_type_name(info),
        );
        code_get_message_size.push(code);

        let code = format!(
            r#"        MessageType::{}(msg) => msg.encode_raw(buf),"#,
            format_message_type_name(info)
        );
        code_encode_raw_message.push(code);

        let code = format!(
            r#"        MessageType::{}(msg) => serde_json::to_string(&msg),"#,
            format_message_type_name(info)
        );
        code_serialize_to_json.push(code);
    });

    let code = format!(
        r#"use bytes::BufMut;
use prost::{{DecodeError, Message}};

#[derive(Clone)]
pub enum MessageType {{
    None,
{}
}}

pub fn get_message_id(message: &MessageType) -> Option<u32> {{
    match message {{
{}
        _ => None,
    }}
}}

pub fn decode_message(message_id: u32, bytes: &[u8]) -> Result<MessageType, DecodeError> {{
    match message_id {{
{}
        _ => Err(DecodeError::new("unknown message id")),
    }}
}}

pub fn encode_message(message: &MessageType) -> Option<(u32, Vec<u8>)> {{
    match message {{
{}
        _ => None,
    }}
}}

pub fn get_message_size(message: &MessageType) -> usize {{
    match message {{
{}
        _ => 0,
    }}
}}

pub fn encode_raw_message(message: &MessageType, buf: &mut impl BufMut) {{
    match message {{
{}
        _ => {{}}
    }}
}}

#[cfg(feature = "serde-serialize")]
pub fn serialize_to_json(message: &MessageType) -> serde_json::Result<String> {{
    match message {{
{}
        _ => Ok("null".into()),
    }}
}}
"#,
        code_message.join("\n"),
        code_get_message_id.join("\n"),
        code_decode_message.join("\n"),
        code_encode_message.join("\n"),
        code_get_message_size.join("\n"),
        code_encode_raw_message.join("\n"),
        code_serialize_to_json.join("\n")
    );

    code
}

fn build(
    proto_file_list: &[impl AsRef<Path>],
    include_list: &[impl AsRef<Path>],
    out_dir: &Path,
) -> io::Result<Vec<MessageInfo>> {
    let package_re = Regex::new(r#"package\s+([\w.]+)"#).unwrap();
    let msg_re = Regex::new(r#"message\s+(\w+)"#).unwrap();
    let id_match_re = Regex::new(r#"enum\s+MsgId\s+"#).unwrap();
    let id_re = Regex::new(r#"Id\s+=\s+(\d+);"#).unwrap();

    let mut messages = Vec::new();

    // 备份并修改协议文件
    for filename in proto_file_list {
        // 当前包名
        let mut package_name = String::new();
        // 当前消息名称
        let mut msg_name = String::new();
        // 当前消息id
        let mut msg_id = 0u32;

        let mut lines = Vec::new();

        let contents = fs::read_to_string(filename)?;

        for line in contents.lines() {
            lines.push(line.to_string());

            let line = line.trim();

            // 获取包名
            if line.starts_with("package") {
                for id_cap in package_re.captures_iter(&line) {
                    package_name = id_cap.get(1).map_or("", |m| m.as_str()).to_string();
                }
            } else if line.starts_with("message ") {
                for msg_cap in msg_re.captures_iter(&line) {
                    msg_name = msg_cap.get(1).map_or("", |m| m.as_str()).to_string();
                }
            } else if (line.starts_with(ANNOTATION_PREFIX) || line.starts_with("enum"))
                && id_match_re.captures(&line).is_some()
            {
                let mut has_id = false;
                for id_cap in id_re.captures_iter(&line) {
                    has_id = true;
                    msg_id = id_cap
                        .get(1)
                        .map_or("", |m| m.as_str())
                        .parse()
                        .expect("message id not a number");
                }
                if has_id {
                    // 注释消息id
                    if !line.starts_with(ANNOTATION_PREFIX) {
                        *lines.last_mut().unwrap() =
                            format!("{}{}", ANNOTATION_PREFIX, lines.last().unwrap());
                    }
                    messages.push(MessageInfo {
                        name: msg_name.clone(),
                        package: package_name.clone(),
                        id: msg_id,
                    });
                }
            }
        }

        fs::write(filename, lines.join("\n"))?;
    }

    fs::write(out_dir.join("message_map.rs"), build_code(&messages))?;

    Config::new()
        .out_dir(out_dir)
        .type_attribute(".", "#[cfg_attr(feature = \"serde-serialize\", derive(serde::Serialize, serde::Deserialize))]")
        .compile_protos(&proto_file_list, &include_list)?;

    // 将生成的 pb.abc_def.rc重命名为abc_def.rc
    for entry in fs::read_dir(out_dir)? {
        let entry = entry?;
        let path = entry.path();

        // 判断文件是否以pb.开头
        if let Some(name) = path.file_stem() {
            if name.to_string_lossy().starts_with("pb.") {
                let new_name = name.to_string_lossy().replace("pb.", "");
                let new_path = out_dir.join(new_name + ".rs");
                // 重命名文件
                fs::rename(path, new_path)?;
            }
        }
    }

    Ok(messages)
}

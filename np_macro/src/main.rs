use std::fs;
use regex::Regex;
use heck::{SnakeCase, CamelCase};


#[derive(Debug)]
struct MessageInfo {
    name: String,
    package: String,
    id: u32
}


fn format_package_name(message_info: &MessageInfo) -> String{
    let package = message_info.package.replace("PB.", "");
    package.to_snake_case()
}

fn format_message_full_type(message_info: &MessageInfo) -> String {
    format!("crate::{}::{}", format_package_name(message_info), message_info.name)
}

fn format_message_type_name(message_info: &MessageInfo) -> String {
    format!("{}_{}", format_package_name(message_info).to_camel_case(), message_info.name)
}

fn main() {
    let package_re = Regex::new(r#"package\s+([\w.]+)"#).unwrap();
    let msg_re = Regex::new(r#"message\s+(\w+)"#).unwrap();
    let id_match_re = Regex::new(r#"enum\s+MsgId\s+"#).unwrap();
    let id_re = Regex::new(r#"Id\s+=\s+(\d+);"#).unwrap();

    let filelist = [
        "D:/rust/npipe/np_macro/src/protos.proto",
        "D:/rust/npipe/np_macro/src/Client_Lobby.proto",
    ];

    let mut messages = Vec::new();

    // 备份并修改协议文件
    filelist.iter().for_each(|filename| {
        // let backup_file = backup_file_name(filename.to_string());
        // let _ = fs::remove_file(&backup_file);
        // fs::copy(filename, &backup_file).unwrap();

        let contents = fs::read_to_string(filename).unwrap();

        // 当前包名
        let mut package_name = String::new();
        // 当前消息名称
        let mut msg_name = String::new();
        // 当前消息id
        let mut msg_id = 0u32;

        let mut lines = Vec::new();
        for line in contents.lines() {
            lines.push(line.to_string());

            let line = line.trim();

            // 获取包名
            if line.starts_with("package") {
                for id_cap in package_re.captures_iter(&line) {
                    package_name = id_cap.get(1).map_or("", |m| m.as_str()).to_string();
                }
            }
            else if line.starts_with("message ") {
                for msg_cap in msg_re.captures_iter(&line) {
                    msg_name = msg_cap
                                        .get(1)
                                        .map_or("", |m| m.as_str())
                                        .to_string();
                    // println!("cur_msg_name:{}", cur_msg_name);
                }
            }
            else if line.starts_with("enum") && id_match_re.captures(&line).is_some() {
                let mut has_id = false;
                for id_cap in id_re.captures_iter(&line) {
                    has_id = true;
                    msg_id = id_cap.get(1).map_or("", |m| m.as_str()).parse().expect("message id not a number");
                }
                if has_id {
                    *lines.last_mut().unwrap() = format!("//{}", lines.last().unwrap());

                    messages.push(MessageInfo {
                        name: msg_name.clone(),
                        package: package_name.clone(),
                        id: msg_id,
                    })
                }
            }
        }
    });

    let mut code_message = Vec::new();
    let mut code_get_message_id = Vec::new();
    let mut code_parse_message = Vec::new();

    messages.iter().for_each(|info| {
        let code = format!("    {}({}),", format_message_type_name(info), format_message_full_type(info));
        code_message.push(code);


        let code = format!("        MessageType::{}(_) => {}u32,", format_message_type_name(info), info.id);
        code_get_message_id.push(code);


        let code = format!(r#"        {}u32 => {{
            match {}::decode(bytes) {{
                Ok(message) => Ok(MessageType::{}(message)),
                Err(err)=> Err(err)
            }}
        }}"#, info.id, format_message_full_type(info), format_message_type_name(info));
        code_parse_message.push(code);
    });



    let code = format!(r#"use prost::{{DecodeError, Message}};

pub enum MessageType {{
    None,
{}
}}

pub fn get_message_id(message: MessageType) ->u32 {{
    match message {{
{}
        _=> panic!("error message")
    }}
}}

pub fn parse_message(message_id: u32, bytes: &[u8]) -> Result<MessageType, DecodeError> {{
    match message_id {{
{}
        _ => Err(DecodeError::new("unknown message id"))
    }}
}}
"#, code_message.join("\n")
  , code_get_message_id.join("\n")
  , code_parse_message.join("\n")
    );

    println!("{}", code);
}
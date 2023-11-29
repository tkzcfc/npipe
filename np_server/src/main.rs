use np_base::Result;

pub fn is_valid(s: String) -> bool {
    use std::collections::HashMap;
    
    let mut pairs:HashMap<char,char> = HashMap::new();
    pairs.insert(')','(');
    pairs.insert(']','[');
    pairs.insert('}','{');
    
    let mut stack = vec![];
    
    for ch in s.chars() {
        if !pairs.contains_key(&ch) {
            stack.push(ch);
        }else{
            if stack.is_empty() || *pairs.get(&ch).unwrap() != *stack.last().unwrap(){
                return false;
            }
            stack.pop();
        }
    }
    
    stack.is_empty()
}


#[tokio::main]
pub async fn main() -> Result<()> {
    // // Open a connection to the mini-redis address.
    // let mut client = Client::connect("127.0.0.1:6379").await?;

    // // Set the key "hello" with value "world"
    // client.set("hello", "world".into()).await?;

    // // Get key "hello"
    // let result = client.get("hello").await?;

    // println!("got value from the server; success={:?}", result.is_some());

    Ok(())
}

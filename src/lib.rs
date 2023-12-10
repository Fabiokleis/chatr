use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct User {
    pub name: String,
    pub address: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Message {
    pub author: User,
    pub content: String,
}

impl Message {
    pub fn new(name: String, address: String, content: String) -> Self {
        Message {
            author: User { name, address },
            content,
        }
    }
}

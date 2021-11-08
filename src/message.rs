pub enum MsgType {
    MSG, // Text message
    ERR, // Error
    LIN, // Login event
    LOU, // Logout event
    ACC, // Accept event (when some message needs to be recognized)
    CHK, // Check event (to see if connection is alive)
}

/// Message delivering protocol
pub struct Message {
    pub msg: MsgType,
    pub from: String,
    pub content: String,
}

impl Message {
    pub fn new(msg: MsgType, from: &str, content: &str) -> Message {
        Message {
            msg: msg,
            from: String::from(from),
            content: String::from(content),
        }
    }

    pub fn from(msg: &str) -> Option<Message> {
        let mut lines = msg.lines();
        if let (Some(command), Some(from), Some(contents)) =
            (lines.next(), lines.next(), lines.next())
        {
            let contents = contents.replace("\\n", "\n");
            match command {
                "MSG" => Some(Message {
                    msg: MsgType::MSG,
                    from: String::from(from),
                    content: String::from(contents),
                }),
                "ERR" => Some(Message {
                    msg: MsgType::ERR,
                    from: String::from(from),
                    content: String::from(contents),
                }),
                "LIN" => Some(Message {
                    msg: MsgType::LIN,
                    from: String::from(from),
                    content: String::from(contents),
                }),
                "LOU" => Some(Message {
                    msg: MsgType::LOU,
                    from: String::from(from),
                    content: String::from(contents),
                }),
                "ACC" => Some(Message {
                    msg: MsgType::ACC,
                    from: String::from(from),
                    content: String::from(contents),
                }),
                "CHK" => Some(Message {
                    msg: MsgType::CHK,
                    from: String::from(from),
                    content: String::from(contents),
                }),
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn to_string(&self) -> String {
        format!(
            "{}\n{}\n{}\n",
            match self.msg {
                MsgType::MSG => "MSG",
                MsgType::ERR => "ERR",
                MsgType::LIN => "LIN",
                MsgType::LOU => "LOU",
                MsgType::ACC => "ACC",
                MsgType::CHK => "CHK",
            },
            self.from,
            self.content.replace("\n", "\\n")
        )
    }
}

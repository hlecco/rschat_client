use std::io::{Error, ErrorKind};
use std::io::{Read, Result, Write};
use std::net::TcpStream;
use std::sync::mpsc;

#[path = "message.rs"] pub mod message;
pub use message::{Message, MsgType};

/// Struct to manage a connection to chat server
pub struct Client {
    connection: Option<TcpStream>,
    pub username: String,
    password: String,
    pub host: String,
    message_receiver: mpsc::Sender<Message>,
    message_sender: mpsc::Receiver<Message>,
    counter: u16,
    warn_level: u8,
}

impl Client {
    /// Creates a new Client instance, with username, password and host unset
    ///
    /// # Arguments
    /// * `sender` - mpsc::Sender, to send fetched data from a TCP stream to the user interface
    /// * `receiver`- mpsc::Receiver that receives messages from the interface and sends to server
    ///
    pub fn new(sender: mpsc::Sender<Message>, receiver: mpsc::Receiver<Message>) -> Client {
        Client {
            connection: None,
            username: String::from(""),
            password: String::from(""),
            host: String::from(""),
            message_receiver: sender,
            message_sender: receiver,
            counter: 0,
            warn_level: 0
        }
    }

    pub fn set_credentials(&mut self, username: &str, password: &str, host: &str) {
        self.username = String::from(username);
        self.password = String::from(password);
        self.host = String::from(host);
    }

    pub fn connect(&mut self) -> Result<()> {
        let mut stream = TcpStream::connect(&self.host)?;
        stream.write(
            Message::new(MsgType::LIN, &self.username, &self.password)
                .to_string()
                .as_bytes(),
        )?;
        let mut buffer = [0; 1024];
        stream.read(&mut buffer)?;

        let msg = String::from_utf8_lossy(&buffer[..]);
        if let Some(msg) = Message::from(&msg) {
            match msg.msg {
                MsgType::ACC => Ok({
                    stream.set_nonblocking(true).unwrap();
                    self.connection = Some(stream);
                }),
                _ => Err(Error::from(ErrorKind::PermissionDenied)),
            }
        } else {
            Err(Error::from(ErrorKind::PermissionDenied))
        }
    }

    /// Tries to read a Message from the buffer
    fn receive_from_server(&mut self) -> Option<Message> {
        let mut buffer = [0; 1024];
        if let Some(stream) = &mut self.connection {
            if let Some(msg) = {
                match stream.read(&mut buffer) {
                    Ok(0) => None,
                    Ok(_) => {
                        let msg = String::from_utf8_lossy(&buffer[..]);
                        match msg.trim().is_empty() {
                            false => Some(msg),
                            true => None,
                        }
                    }
                    Err(_) => None,
                }
            } {
                return Message::from(&msg)
            }
        }
        None
    }

    /// Tries to receive a message from the interface and send it to the server
    fn send_text_to_server(&mut self) -> Result<usize> {
        if let Some(stream) = &mut self.connection {
            if let Ok(mut msg) = self.message_sender.try_recv() {
                msg.from = String::from(&self.username);
                let written = stream.write(msg.to_string().as_bytes())?;
                match &msg.msg {
                    MsgType::ERR => return Err(Error::from(ErrorKind::ConnectionAborted)),
                    MsgType::LOU => return Err(Error::from(ErrorKind::ConnectionAborted)),
                    _ => return Ok(written)
                }
            }
        }
        Ok(0)
    }

    /// Sends any message to server
    fn send_message_to_server(&mut self, msg: Message) -> Result<usize> {
        match &mut self.connection {
            Some(stream) => stream.write(msg.to_string().as_bytes()),
            None => Ok(0)
        }
    }

    /// Sends some message to the user interface
    pub fn send_message_to_ui(&mut self, msg: Message) {
        self.message_receiver.send(msg).expect("mpsc channel broke");
    }


    /// Listens to a single message from the server and sends a single mesassage from the buffer
    pub fn listen(&mut self) -> Result<usize> {
        if self.counter < 500 {
            self.counter = self.counter+1;
        }
        else {
            self.counter = 0;
            self.warn_level += 1;
            self.send_message_to_server(Message::new(MsgType::CHK, &self.username, ""))?;
        }
        self.send_text_to_server()?;
        if let Some(msg) = self.receive_from_server() {
            match &msg.msg {
                MsgType::MSG => Ok({self.message_receiver.send(msg).expect("mpsc channel failed"); 0}),
                MsgType::CHK => self.send_message_to_server(Message::new(MsgType::ACC, &self.username, "")),
                MsgType::ACC => Ok({self.warn_level = 0; 0}),
                MsgType::ERR => Err(Error::from(ErrorKind::ConnectionAborted)),
                MsgType::LOU => Err(Error::from(ErrorKind::ConnectionAborted)),
                _ => Ok(0)
            }
        }
        else {
            match self.warn_level > 5 {
                false => Ok(0),
                true => Err(Error::from(ErrorKind::ConnectionAborted))
            }
        }
    }
}

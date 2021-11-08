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
    fn send_text_to_server(&mut self) {
        if let Some(stream) = &mut self.connection {
            if let Ok(mut msg) = self.message_sender.try_recv() {
                msg.from = String::from(&self.username);
                stream.write(msg.to_string().as_bytes());
            }
        }
    }

    /// Listens to a single message from the server and sends a single mesassage from the buffer
    pub fn listen(&mut self) -> Result<()> {
        self.send_text_to_server();
        if let Some(msg) = self.receive_from_server() {
            match &msg.msg {
                MsgType::MSG => Ok({self.message_receiver.send(msg);}),
                MsgType::CHK => Ok({}),
                MsgType::ERR => Err(Error::from(ErrorKind::ConnectionAborted)),
                MsgType::LOU => Err(Error::from(ErrorKind::ConnectionAborted)),
                _ => Ok(())
            }
        }
        else {
            Ok(())
        }
    }
}

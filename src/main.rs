use std::rc::Rc;
use std::cell::{RefCell, Cell};
use std::sync::mpsc;
use std::{thread, time};
use std::process;

use rschat_client::{Client, Message, MsgType};

use cursive::traits::*;
use cursive::views::{
    Button, Dialog, DummyView, EditView, LinearLayout, Panel, ScrollView, TextArea, TextView,
};
use cursive::{Cursive, CursiveRunnable};

mod consts;

use term_size;

/// Creates a Cursive interface for logging in
///
/// # Arguments
///
/// * `client: &mut Client`: where to take credentials from
fn login_form(client: &mut Client) -> Option<(String, String, String)> {
    let host = Rc::new(RefCell::new(String::from(&client.host)));
    let username = Rc::new(RefCell::new(String::from(&client.username)));
    let password = Rc::new(RefCell::new(String::new()));

    let quit = Rc::new(Cell::new(false));

    {
        let host = host.clone();
        let password = password.clone();
        let username = username.clone();

        let quit = quit.clone();

        let mut siv = CursiveRunnable::default();
        let login_form = LinearLayout::vertical()
            .child(
                Panel::new(EditView::new().content(host.clone().borrow().to_string()).with_name("host"))
                    .title("Host")
                    .title_position(cursive::align::HAlign::Left),
            )
            .child(
                Panel::new(EditView::new().content(username.clone().borrow().to_string()).with_name("username"))
                    .title("Username")
                    .title_position(cursive::align::HAlign::Left),
            )
            .child(
                Panel::new(EditView::new().content(password.clone().borrow().to_string()).secret().with_name("password"))
                    .title("Password")
                    .title_position(cursive::align::HAlign::Left),
            )
            .child(
                LinearLayout::horizontal()
                    .child(Button::new("Login", move |s: &mut Cursive| {
                        let host = host.clone();
                        host.borrow_mut().clear();
                        host.borrow_mut().push_str(&s
                            .call_on_name("host", |v: &mut EditView| v.get_content())
                            .unwrap()
                            .to_string()
                            );
                        let username = username.clone();
                        username.borrow_mut().clear();
                        username.borrow_mut().push_str(&s
                            .call_on_name("username", |v: &mut EditView| v.get_content())
                            .unwrap()
                            .to_string()
                            );
                        let password = password.clone();
                        password.borrow_mut().clear();
                        password.borrow_mut().push_str(&s
                            .call_on_name("password", |v: &mut EditView| v.get_content())
                            .unwrap()
                            .to_string()
                            );
                        s.quit();
                    }))
                    .child(DummyView)
                    .child(Button::new("Quit", move |s| { quit.clone().set(true); s.quit(); }))
            );

        siv.add_layer(login_form);
        siv.run();
    }

    let x = (username.borrow().to_string(), password.borrow().to_string(), host.borrow().to_string());
    match quit.get() {
        true => None,
        false => Some(x)
    }
}

/// Keeps trying to log in until success
///
/// # Arguments
///
/// * `client: &mut Client` - Client to connect
fn login(client: &mut Client) {
    loop {
        if let Some((username, password, host)) = login_form(client) {
            client.set_credentials(&username, &password, &host);
            match client.connect() {
                Ok(_) => { break; }
                Err(_) => { continue; }
            }
        }
        else {
            process::exit(0);
        }
    }
}

fn main() {
    let (listener_sender, listener_receiver) = mpsc::channel();
    let (writer_sender, writer_receiver) = mpsc::channel();

    let mut siv = cursive::default();
    siv.add_layer(layout_main(writer_sender));

    let mut client = Client::new(listener_sender, writer_receiver);
    loop {
        login(&mut client);

        let handle = thread::spawn(move || {
            loop {
                match client.listen() {
                    Ok(_) => {thread::sleep(time::Duration::from_millis(10));},
                    Err(_) => {break;}
                }
            }
            client.send_message_to_ui(Message::new(MsgType::ERR, "", "thread stopped"));
            client
        });


        let mut runner = siv.runner();

        runner.refresh();
        while runner.is_running() {
            if let Ok(msg) = listener_receiver.try_recv() {
                match msg.msg {
                    MsgType::MSG => {
                        receive_message(&mut *runner, &msg.from, &msg.content);
                    }
                    MsgType::ERR => { break; }
                    _ => (),
                }
                runner.refresh();
            }
            runner.step();
        }

        client = handle.join().unwrap();
        println!("joined");
    }
}

/// Cursive layout with chat log and text box
///
/// # Arguments
///
/// * `sender` - a mpsc Sender that sends written on the text box to a connection
fn layout_main(sender: mpsc::Sender<Message>) -> LinearLayout {
    let (width, height) = term_size::dimensions().unwrap();
    let width = width - 2 * consts::PADDING_X;
    let height = height - 2 * consts::PADDING_Y;

    let log = ScrollView::new(
        LinearLayout::vertical()
            .with_name("log")
            .fixed_width(width / 2),
    )
    .scroll_strategy(cursive::view::ScrollStrategy::StickToBottom)
    .fixed_height(height);

    let write = TextArea::new()
        .with_name("write")
        .fixed_width(width / 2)
        .min_height(5)
        .max_height(8);

    let sender = Rc::new(sender);
    let sender_quit = sender.clone();
    let sender_send = sender.clone();

    let dialog = (Dialog::around(write)
        .button("Send", move |s| send_text(s, &sender_send))
        .button("Quit", move |_| { sender_quit.send(Message::new(MsgType::LOU, "", "")).unwrap(); })
        .with_name("msg_dialog"))
    .fixed_height(height);

    LinearLayout::horizontal().child(log).child(dialog)
}

/// Consumes text on text box and sends it
fn send_text(s: &mut Cursive, sender: &mpsc::Sender<Message>) {
    let msg = s
        .call_on_name("write", |view: &mut TextArea| {
            let msg = view.get_content().to_string();
            view.set_content("");
            msg
        })
        .unwrap();
    let send_message = Message::new(MsgType::MSG, "", &msg);
    sender.send(send_message).expect("Broken mpsc channel");
}

/// Creates a new entry on the text box
///
/// # Arguments
///
/// * `s: &mut Cursive` - Cursive instance with the text log
/// * `from: &str` - who sent the message
/// * `contents: &str` - contents of message
fn receive_message(s: &mut Cursive, from: &str, contents: &str) {
    s.call_on_name("log", |view: &mut LinearLayout| {
        view.add_child(
            Panel::new(TextView::new(contents))
                .title(from)
                .title_position(cursive::align::HAlign::Left),
        );
    });
}

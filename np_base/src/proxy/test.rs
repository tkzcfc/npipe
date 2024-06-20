// use std::sync::{Arc, Mutex};
// use tokio::time::{sleep, Duration};
// use tokio::sync::mpsc::UnboundedSender;
// use std::collections::HashMap;
//
// pub type CreateSessionDelegateCallback = Box<dyn Fn() -> Box<dyn SessionDelegate> + Send + Sync>;
//
// pub async fn run_server(
//     on_create_session_delegate_callback: CreateSessionDelegateCallback,
// ) {
//     loop {
//         sleep(Duration::from_secs(1)).await;
//         let delegate = on_create_session_delegate_callback();
//     }
// }
//
// type SenderMap = Arc<Mutex<HashMap<u32, UnboundedSender<WriterMessage>>>>;
//
// pub struct Inlet {
//     sender_map: SenderMap,
// }
//
// impl Inlet {
//     pub async fn start(&self) {
//         let sender_map = self.sender_map.clone();
//
//         let create_session_delegate_func: CreateSessionDelegateCallback = Box::new(move || {
//             Box::new(InletSession::new(sender_map.clone())) as Box<dyn SessionDelegate>
//         });
//
//         run_server(create_session_delegate_func).await;
//     }
// }
//
// pub trait SessionDelegate {
//     // trait methods here
// }
//
// pub struct InletSession {
//     sender_map: SenderMap,
// }
//
// impl InletSession {
//     pub fn new(sender_map: SenderMap) -> Self {
//         Self { sender_map }
//     }
// }
//
// impl SessionDelegate for InletSession {
//     // implementation here
// }
//
// // Dummy types to make the code compile
// pub struct WriterMessage;

use tokio::sync::oneshot;

// pub struct Fetcher {
//     sender: oneshot::Sender<()>,
//     receiver: oneshot::Receiver<()>,
// }

// impl Fetcher {
//     pub fn new() -> Self {
//         let (sender, receiver) = oneshot::channel();

//         receiver.try_recv()

//         Self {
//             sender,
//             receiver,
//         }
//     }
// }

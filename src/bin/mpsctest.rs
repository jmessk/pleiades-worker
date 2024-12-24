use tokio::sync::{mpsc, oneshot};

#[tokio::main]
async fn main() {
    let (sender, mut receiver) = mpsc::channel(10);

    tokio::spawn(async move {
        for i in 0..10 {
            let (tx, rx) = oneshot::channel();

            sender.send((i, tx)).await.unwrap();
            tokio::spawn(async move {
                let i = rx.await.unwrap();
                println!("{}: acked", i);
            });
        }
    });

    while let Some((i, tx)) = receiver.recv().await {
        println!("{}: received", i);
        tx.send(i).unwrap();
    }
}

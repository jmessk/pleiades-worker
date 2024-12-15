#[tokio::main]
async fn main() {
    // let (tx, rx) = tokio::sync::mpsc::channel(32);

    // tokio::spawn(async move {
    //     let mut rx = rx;
    //     while let Some(message) = rx.recv().await {
    //     }
    // });

    // for _ in 0..100000 {
    //     tx.send([5u8; 256]).await.unwrap();
    // }

    // println!("done");

    /////////

    let (tx, rx) = tokio::sync::mpsc::channel(32);

    tokio::spawn(async move {
        let mut rx = rx;
        while let Some(message) = rx.recv().await {}
    });

    for _ in 0..1000000 {
        let tx = tx.clone();
        tokio::spawn(async move {
            tx.send([5u8; 256]).await.unwrap();
        });
    }

    println!("done");
}

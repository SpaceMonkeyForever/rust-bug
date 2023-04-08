use std::thread;
use chrono::Utc;
use tokio::sync::watch::Receiver;
use std::time::Duration;
use tokio::sync::watch;
use tokio::time::sleep;

pub fn get_now_formatted() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S.%3f").to_string()
}

#[tokio::main]
async fn main() {
    let mut rx = subscribe_orderbook().await;

    // simple thread that reads from the stream
    tokio::spawn(async move {
        loop {
            println!("{}: Trying to read..", get_now_formatted());
            rx.changed().await.expect("TODO: panic message");
            println!("NEVER REACHES HERE UNLESS WE COMMENT OUT AWAIT LINE IN SENDER");
            println!("{}: Received: {:?}", get_now_formatted(), rx.borrow().clone());
        }
    });

    loop {
        sleep(Duration::from_millis(1000)).await;
    }
}

pub async fn subscribe_orderbook() -> Receiver<i32> {
    // this is where the issue will happen:
    let (channel_tx, channel_rx) = watch::channel(1);

    tokio::spawn( async move {
        let mut counter = 0;
        loop {
            counter += 1;
            let result = channel_tx.send(counter);
            if result.is_err() {
                eprintln!("{} Failed to send fair value to main thread: {}", get_now_formatted(), result.err().unwrap());
            }
            else {
                println!("{} SENT {:?}", get_now_formatted(), counter);
            }

            // NOTE: commenting out this pointless await fixes the problem!
            // sleep(Duration::from_millis(0)).await;

            thread::sleep(Duration::from_millis(1000));
        }
    });
   channel_rx
}
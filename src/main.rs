use std::cmp::max;
use std::collections::HashMap;
use std::env;
use chrono::Utc;
use binance::api::Binance;
use binance::market::Market;
use binance::websockets::*;
use std::sync::atomic::{AtomicBool};
use tokio::sync::watch::Receiver;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::{thread};
use std::sync::atomic::Ordering;
use binance::model::{DepthOrderBookEvent};
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
            println!("{}: Running loop", get_now_formatted());
            // wait for a change in either stream
            tokio::select! {
                //timeout
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)) => {
                    println!("{}: waiting..", get_now_formatted());
                },
                //
                _ = rx.changed() => {
                    println!("NEVER REACHES HERE UNLESS WE COMMENT OUT AWAIT LINE IN SENDER");
                    println!("{}: Received: {:?}", get_now_formatted(), rx.borrow().clone());
                },
            }
        }
    });


    loop {
        sleep(Duration::from_millis(1000)).await;
    }
}

pub async fn subscribe_orderbook() -> Receiver<i32> {
    let keep_running = AtomicBool::new(true);
    // this is where the issue will happen:
    let (channel_tx, channel_rx) = watch::channel(1);
    // just a bridge to be able to process incoming data on an async thread because the websocket callback can only be sync
    let (queue_tx, mut queue_rx) = crossbeam::channel::bounded(100);

    tokio::spawn( async move {
        let mut counter = 0;
        loop {
            counter += 1;
            let data: DepthOrderBookEvent = queue_rx.recv().expect("Failed waiting for orderbook update from websocket thread");

            let result = channel_tx.send(counter);
            if result.is_err() {
                eprintln!("{} Failed to send fair value to main thread: {}", get_now_formatted(), result.err().unwrap());
            }
            else {
                println!("{} SENT {:?}", get_now_formatted(), counter);
            }

            // NOTE: commenting out this pointless await fixes the problem!
            // sleep(Duration::from_millis(0)).await;
        }
    });

    tokio::spawn( async move {
        loop {
            // This callback has to be sync so we put messages in a queue and process in a separate task
            let mut web_socket: WebSockets<'_> = WebSockets::new(|event: WebsocketEvent| {
                match event {
                    WebsocketEvent::DepthOrderBook(data) => {
                        // If we send over "tx" here directly, then the same issue happens
                        queue_tx.send(data).expect("Failed to send orderbook update to processor thread");
                    },
                    _ => {}
                }

                Ok(())
            });

            web_socket.connect_multiple_streams(&vec!["BTCUSDT@depth@100ms".to_string()]).unwrap(); // check error

            if let Err(e) = web_socket.event_loop(&keep_running) {
                println!("Error: {:?}", e);
            }

            println!("event_loop finished");
            web_socket.disconnect().unwrap();
        }
    });

    channel_rx
}
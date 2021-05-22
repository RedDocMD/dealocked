use std::error::Error;
use std::sync::{Arc, Condvar, Mutex};
use tokio::sync::oneshot;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // This is a channel, for sender to signal reciever that it sees flag to be true.
    let (tx, mut rx) = oneshot::channel::<i64>();

    // These the Signal's - contains a bool guarded by a Mutex and a Condvar
    let sig_send = Arc::new(Signal::new());
    let sig_recv = Arc::clone(&sig_send);

    let sender = tokio::spawn(async move {
        // All this does is it waits for the flag to turn true.
        // Then it sends a value over the channel.
        let mut flag = sig_recv.flag.lock().unwrap();
        while !*flag {
            println!("Waiting for flag to turn true ...");
            flag = sig_recv.cvar.wait(flag).unwrap();
        }
        let _ = tx.send(100);
    });

    let reciever = tokio::spawn(async move {
        // Al this does is it waits to recieve a value over the channel.
        // Then it sets the flag to be true, and notifies other "threads" using the
        // condvar that it has set the flag.
        println!("Trying to recieve value ...");
        if let Ok(data) = rx.try_recv() {
            println!("Recieved {} over channel", data);
            let mut flag = sig_send.flag.lock().unwrap();
            *flag = true;
            println!("Signalling flag has turned true ...");
            sig_send.cvar.notify_one();
        }
    });

    // I am pretty sure you can see why a deadlock occurs.

    sender.await?;
    reciever.await?;
    Ok(())
}

struct Signal {
    flag: Mutex<bool>,
    cvar: Condvar,
}

impl Signal {
    fn new() -> Self {
        Self {
            flag: Mutex::new(false),
            cvar: Condvar::new(),
        }
    }
}

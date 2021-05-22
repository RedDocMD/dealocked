use std::error::Error;
use std::sync::{Arc, Condvar, Mutex};
use tokio::sync::oneshot;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let (tx, mut rx) = oneshot::channel::<i64>();
    let sig_send = Arc::new(Signal::new());
    let sig_recv = Arc::clone(&sig_send);

    let sender = tokio::spawn(async move {
        let mut flag = sig_recv.flag.lock().unwrap();
        while !*flag {
            println!("Waiting for flag to turn true ...");
            flag = sig_recv.cvar.wait(flag).unwrap();
        }
        let _ = tx.send(100);
    });

    let reciever = tokio::spawn(async move {
        println!("Trying to recieve value ...");
        if let Ok(data) = rx.try_recv() {
            println!("Recieved {} over channel", data);
            let mut flag = sig_send.flag.lock().unwrap();
            *flag = true;
            println!("Signalling flag has turned true ...");
            sig_send.cvar.notify_one();
        }
    });

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

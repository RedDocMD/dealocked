use log::debug;
use simplelog::{Config, LevelFilter, SimpleLogger};
use std::cell::Cell;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::thread;
use std::time::Duration;
use tokio::runtime::Builder;
use tokio::sync::oneshot;
use tokio::sync::Notify;

fn main() {
    let runtime = Builder::new_current_thread().build().unwrap();
    runtime.block_on(async {
        top().await.unwrap();
    });
}

async fn top() -> Result<(), Box<dyn Error>> {
    SimpleLogger::init(LevelFilter::Debug, Config::default())?;

    // This is a channel, for sender to signal reciever that it sees flag to be true.
    let (tx, mut rx) = oneshot::channel::<i64>();

    let notify_send = Arc::new(Notify::new());
    let notify_recv = Arc::clone(&notify_send);

    // let sender = tokio::spawn(async move {
    //     debug!("Waiting to be notified ...");
    //     notify_recv.notified().await;
    //     debug!("Sending value over channel");
    //     let _ = tx.send(100);
    // });
    let sender = tokio::spawn(Sender::new(notify_recv, tx));

    let reciever = tokio::spawn(async move {
        debug!("Trying to recieve value ...");
        if let Ok(data) = rx.try_recv() {
            debug!("Recieved {} over channel", data);
            debug!("Notifying other ...");
            notify_send.notify_one();
        }
    });

    // I am pretty sure you can see why a deadlock occurs.

    sender.await?;
    reciever.await?;
    Ok(())
}

struct Sender {
    notify_recv: Arc<Notify>,
    tx: Option<oneshot::Sender<i64>>,
    state: Cell<State>,
}

#[derive(Copy, Clone)]
enum State {
    Started,
    Waiting,
    Done,
}

impl Sender {
    fn new(notify_recv: Arc<Notify>, tx: oneshot::Sender<i64>) -> Self {
        Self {
            notify_recv,
            tx: Some(tx),
            state: Cell::new(State::Started),
        }
    }
}

impl Future for Sender {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        use State::*;

        loop {
            match self.state.get() {
                Started => {
                    debug!("Waiting to be notified ...");
                    self.state.set(Waiting);
                }
                Waiting => {
                    debug!("Sender polled to try to get notified");
                    let notified = self.notify_recv.notified();
                    match Box::pin(notified).as_mut().poll(cx) {
                        Poll::Ready(()) => self.state.set(Done),
                        Poll::Pending => {
                            let waker = cx.waker().clone();
                            thread::spawn(move || {
                                thread::sleep(Duration::from_millis(400));
                                waker.wake_by_ref();
                            });
                            return Poll::Pending;
                        }
                    };
                }
                Done => {
                    debug!("Sending value over channel ...");
                    let tx = std::mem::take(&mut self.tx);
                    let _ = tx.unwrap().send(100);
                    return Poll::Ready(());
                }
            }
        }
    }
}

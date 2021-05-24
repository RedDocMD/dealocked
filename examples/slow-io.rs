use futures::{
    future::BoxFuture,
    task::{waker_ref, ArcWake},
    Future,
};
use log::{debug, info};
use simplelog::{Config, LevelFilter, SimpleLogger};
use std::error::Error;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::{Context, Poll};
use tempfile::tempfile;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

fn main() -> Result<(), Box<dyn Error>> {
    let (executor, spawner) = new_executor_and_spawner();
    SimpleLogger::init(LevelFilter::Debug, Config::default())?;
    let file = tempfile().expect("Failed to create tempfile");
    let mut file = File::from_std(file);
    spawner.spawn(async move {
        const URL: &'static str = "http://localhost:4000";
        debug!("Getting {}", URL);
        let response = reqwest::get(URL).await.unwrap();
        let contents = response.bytes().await.unwrap();
        file.write_all(&contents)
            .await
            .expect("Failed to write to file");
    });
    drop(spawner);
    executor.run();
    Ok(())
}

struct Task {
    future: Mutex<Option<BoxFuture<'static, ()>>>,
    task_sender: mpsc::SyncSender<Arc<Task>>,
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        info!("Trying to poll task after wake ...");
        let clone = arc_self.clone();
        arc_self
            .task_sender
            .send(clone)
            .expect("Failed to push task into channel");
    }
}

struct Executor {
    ready_queue: mpsc::Receiver<Arc<Task>>,
}

#[derive(Clone)]
struct Spawner {
    task_sender: mpsc::SyncSender<Arc<Task>>,
}

fn new_executor_and_spawner() -> (Executor, Spawner) {
    const BUF_SIZE: usize = 10_000;
    let (tx, rx) = mpsc::sync_channel(BUF_SIZE);
    (Executor { ready_queue: rx }, Spawner { task_sender: tx })
}

impl Spawner {
    fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + 'static + Send,
    {
        let task = Task {
            future: Mutex::new(Some(Box::pin(future))),
            task_sender: self.task_sender.clone(),
        };
        self.task_sender
            .send(Arc::new(task))
            .expect("Failed to spawn task");
    }
}

impl Executor {
    fn run(&self) {
        while let Ok(task) = self.ready_queue.recv() {
            info!("Got a task");
            let mut future_slot = task.future.lock().unwrap();
            if let Some(mut future) = future_slot.take() {
                let waker = waker_ref(&task);
                let mut context = Context::from_waker(&waker);
                if let Poll::Pending = future.as_mut().poll(&mut context) {
                    *future_slot = Some(future);
                }
            }
        }
    }
}

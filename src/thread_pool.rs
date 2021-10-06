use std::sync::{
    mpsc::{self, Receiver, Sender},
    Arc, Mutex,
};
use std::thread::{Builder, JoinHandle};

pub struct ThreadPool<T> {
    sender: Sender<Message<T>>,
    _workers: Vec<Worker>,
}

enum Message<T> {
    Task(Task<T>),
    Terminate,
}

impl<T> ThreadPool<T>
where
    T: Send + 'static,
{
    pub fn new(num_workers: usize, f: impl Fn() -> T) -> Self {
        let (sender, receiver) = mpsc::channel::<Message<T>>();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(num_workers);

        for id in 0..workers.capacity() {
            let receiver = Arc::clone(&receiver);
            let worker = Worker::new(id, f(), receiver);
            workers.push(worker);
        }

        ThreadPool {
            sender,
            _workers: workers,
        }
    }

    pub fn execute(&self, f: impl FnOnce(&mut T) + Send + 'static) {
        let message = Message::Task(Task::new(f));
        self.sender.send(message).unwrap();
    }
}

impl<T> Drop for ThreadPool<T> {
    fn drop(&mut self) {
        for _ in &self._workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        for mut worker in self._workers.drain(..) {
            if let Some(handle) = worker.handle.take() {
                handle.join().unwrap();
            }
        }
    }
}

struct Worker {
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    fn new<T>(id: usize, resource: T, receiver: Arc<Mutex<Receiver<Message<T>>>>) -> Self
    where
        T: Send + 'static,
    {
        let thread_builder = Builder::new().name(format!("{}", id));
        let handle = thread_builder
            .spawn(move || {
                let mut resource = resource;
                loop {
                    match receiver.lock().unwrap().recv().unwrap() {
                        Message::Task(task) => task.run(&mut resource),
                        Message::Terminate => break,
                    }
                }
            })
            .unwrap();

        let handle = Some(handle);
        Worker { handle }
    }
}

struct Task<T> {
    inner: Box<dyn FnOnce(&mut T) + Send + 'static>,
}

impl<T> Task<T> {
    fn new(f: impl FnOnce(&mut T) + Send + 'static) -> Self {
        Task { inner: Box::new(f) }
    }

    fn run(self, resource: &mut T) {
        (self.inner)(resource)
    }
}

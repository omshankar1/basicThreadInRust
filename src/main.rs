use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time;

pub struct Threadpool {
    _handle: Vec<std::thread::JoinHandle<()>>,
    sender: Sender<Box<dyn FnOnce() + Send>>,
}

impl Threadpool {
    pub fn new(num_threads: u8) -> Self {
        // FnOnce(or FnMut) because we would like to modify the atomic variable
        //        that we send in closure
        // dyn -> FnOnce is a trait and we don't know its concrete impl
        // Box -> to satisfy compiler because it can't figure out the size of FnOnce that we pass in
        // Send -> so that recv and send implements 'send' meaning it can be send to multiple
        // threads
        let (send, recv) = channel::<Box<dyn FnOnce() + Send>>();
        let recv = Arc::new(Mutex::new(recv));
        let mut handle: Vec<std::thread::JoinHandle<()>> = Vec::new();
        for _ in 0..num_threads {
            let recv = recv.clone();
            let _handle = std::thread::spawn(move || loop {
                let work = match recv.lock().unwrap().recv() {
                    Ok(work) => work,
                    Err(e) => {
                        println!("Error: {}", e);
                        break;
                    }
                };
                println!("Starting on task");
                work();
                println!("Finished executing the task");
            });
            handle.push(_handle);
        }
        Self {
            _handle: handle,
            sender: send,
        }
    }

    // static because the lifetime of vairable passed in the function
    // lives as long as the variable in the pub fn main()
    //  In this case the atomic vairable that is passed is cloned and moved to

    pub fn execute<F: FnOnce() + Send + 'static>(&self, work: F) {
        self.sender.send(Box::new(work)).unwrap();
    }
}

fn main() {
    let pool = Threadpool::new(8);
    let num = AtomicU32::new(0);
    let nref = Arc::new(num);
    let nref_clone = Arc::clone(&nref);

    let work = move || {
        nref.fetch_add(1, Ordering::SeqCst);
        println!("thread 1: {:?}", nref.load(Ordering::Relaxed));
    };
    pool.execute(work.clone());
    pool.execute(work.clone());
    pool.execute(work.clone());
    pool.execute(work);
    nref_clone.fetch_add(1, Ordering::SeqCst);
    println!("Main thread: {:?}", nref_clone.load(Ordering::Relaxed));
    // Really bad way to keep the main thread alive
    std::thread::sleep(std::time::Duration::from_secs(3));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        let pool = Threadpool::new(8);
        pool.execute(|| println!("Hello from thread"));
    }
}

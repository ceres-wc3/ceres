use std::cell::RefCell;
use std::sync::mpsc::*;

pub enum Message {
    ChildTerminated,
    SomethingElse,
}

struct Context {
    rx: Option<Receiver<Message>>,
    tx: Option<Sender<Message>>,
}

impl Default for Context {
    fn default() -> Context {
        let (tx, rx) = channel();
        Context { tx: Some(tx), rx: Some(rx) }
    }
}

thread_local! {
    static CONTEXT: RefCell<Context> = RefCell::new(Context::default())
}

pub fn get_event_loop_tx() -> Sender<Message> {
    CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        ctx.tx.as_ref().unwrap().clone()
    })
}

pub fn wait_on_evloop() {
    CONTEXT.with(|ctx| {
        let mut borrowed_ctx = ctx.borrow_mut();
        let rx = borrowed_ctx
            .rx
            .take()
            .expect("evloop recv must be available");
        // no more tx for you!
        let tx = borrowed_ctx.tx.take();
        drop(tx);
        drop(borrowed_ctx);

        while let Ok(message) = rx.recv() {
            if let Message::ChildTerminated = message {
                break;
            }
        }
    })
}

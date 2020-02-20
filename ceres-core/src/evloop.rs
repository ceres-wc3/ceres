use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::*;

use anyhow::Context as _;
use rlua::Lua;
use rlua::prelude::{LuaContext, LuaError};

use crate::handle_lua_result;

pub enum Message {
    ChildTerminated,
    LuaRun(Box<dyn Send + Sync + Fn(LuaContext) -> Result<(), LuaError>>),
}

struct Context {
    rx: Option<Receiver<Message>>,
    tx: Option<Sender<Message>>,
}

impl Default for Context {
    fn default() -> Context {
        let (tx, rx) = channel();
        Context {
            tx: Some(tx),
            rx: Some(rx),
        }
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

pub fn wait_on_evloop(lua: Rc<Lua>) {
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
            match message {
                Message::ChildTerminated => break,
                Message::LuaRun(callback) => {
                    let should_continue = lua.context(|ctx| {
                        let result = callback(ctx);

                        if result.is_err() {
                            println!("[ERROR] An error occured inside the event loop. The event loop will terminate.");
                            handle_lua_result(result.context("evloop callback failed"));
                            return false;
                        }

                        true
                    });

                    if !should_continue {
                        break;
                    }
                }
            }
        }
    })
}

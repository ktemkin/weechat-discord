use crate::refcell::RefCell;
use std::{
    borrow::Cow,
    collections::{HashSet, VecDeque},
    rc::Rc,
};
use weechat::buffer::BufferHandle;

pub trait WeechatMessage<I, S> {
    /// Format the message into the prefix and body
    fn render(&self, state: &mut S) -> (String, String);
    fn tags(&self, state: &mut S) -> HashSet<Cow<'static, str>>;
    fn timestamp(&self, state: &mut S) -> i64;
    fn id(&self, state: &mut S) -> I;
}

pub struct MessageRenderer<M: WeechatMessage<I, S> + Clone, I: Eq, S> {
    messages: Rc<RefCell<VecDeque<M>>>,
    buffer_handle: Rc<BufferHandle>,
    state: Rc<RefCell<S>>,
    max_buffer_messages: Rc<usize>,
    last_read_id: Rc<RefCell<Option<I>>>,
    // the oldest timestamp currently rendered
    oldest_rendered_timestamp: Rc<RefCell<Option<i64>>>,
}

impl<M: WeechatMessage<I, S> + Clone, I: Eq, S> Clone for MessageRenderer<M, I, S> {
    fn clone(&self) -> Self {
        Self {
            messages: Rc::clone(&self.messages),
            buffer_handle: Rc::clone(&self.buffer_handle),
            state: Rc::clone(&self.state),
            max_buffer_messages: Rc::clone(&self.max_buffer_messages),
            last_read_id: Rc::clone(&self.last_read_id),
            oldest_rendered_timestamp: Rc::clone(&self.oldest_rendered_timestamp),
        }
    }
}

impl<M: WeechatMessage<I, S> + Clone, I: Eq, S> MessageRenderer<M, I, S> {
    pub fn new(buffer_handle: Rc<BufferHandle>, max_buffer_messages: usize, state: S) -> Self {
        Self {
            buffer_handle,
            state: Rc::new(RefCell::new(state)),
            max_buffer_messages: Rc::new(max_buffer_messages),
            messages: Rc::new(RefCell::new(VecDeque::new())),
            last_read_id: Rc::new(RefCell::new(None)),
            oldest_rendered_timestamp: Rc::new(RefCell::new(None)),
        }
    }

    pub fn buffer_handle(&self) -> Rc<BufferHandle> {
        self.buffer_handle.clone()
    }

    pub fn messages(&self) -> Rc<RefCell<VecDeque<M>>> {
        self.messages.clone()
    }

    pub fn state(&self) -> Rc<RefCell<S>> {
        self.state.clone()
    }

    pub fn set_last_read_id(&self, id: I) {
        *self.last_read_id.borrow_mut() = Some(id);
    }

    fn print_msg(&self, msg: &M, log: bool) {
        let buffer = self
            .buffer_handle
            .upgrade()
            .expect("message renderer outlived buffer");

        let mut state = self.state.borrow_mut();
        let (prefix, suffix) = msg.render(&mut state);
        let mut tags = msg.tags(&mut state);
        if !log {
            tags.insert("no_log".into());
        }

        let tags: Vec<_> = tags.iter().map(Cow::as_ref).collect();
        buffer.print_date_tags(
            msg.timestamp(&mut state),
            &tags,
            &format!("{}\t{}", prefix, suffix),
        );
    }

    pub fn redraw_buffer(&self) {
        tracing::trace!("Redrawing buffer");
        self.buffer_handle
            .upgrade()
            .expect("message renderer outlived buffer")
            .clear();

        let last_read_id = self.last_read_id.borrow();
        self.render_history(self.messages.borrow().iter().rev(), &last_read_id);
    }

    pub fn add_msg(&self, msg: M) {
        self.print_msg(&msg, true);

        let mut messages = self.messages.borrow_mut();
        messages.push_front(msg);
        messages.truncate(*self.max_buffer_messages);
    }

    pub fn add_bulk_msgs(&self, msgs: impl DoubleEndedIterator<Item = M>) {
        let mut messages = self.messages.borrow_mut();
        messages.extend(msgs.rev().take(*self.max_buffer_messages));
        messages.truncate(*self.max_buffer_messages);

        let last_read_id = self.last_read_id.borrow();
        self.render_history(messages.iter().rev(), &last_read_id);
    }

    fn render_history<'a>(
        &'a self,
        messages: impl Iterator<Item = &'a M>,
        last_read_id: &Option<I>,
    ) {
        // TODO: Can the oldest message check be optimized to avoid copying?
        let messages = messages.collect::<Vec<_>>();
        let oldest_timestamp_to_render = {
            let mut state = self.state.borrow_mut();
            messages.iter().map(|msg| msg.timestamp(&mut state)).min()
        };

        let buffer = self.buffer_handle.upgrade().unwrap();

        // If there is no oldest rendered timestamp, then no messages have been rendered,
        // and we don't need to clear
        if let Some(&oldest_rendered_timestamp) = self.oldest_rendered_timestamp.borrow().as_ref() {
            if let Some(oldest_timestamp_to_render) = oldest_timestamp_to_render {
                // if the oldest message to render is older than our current oldest rendered timestamp,
                // then we need to prepend the new message, which requires we clear the buffer first
                if oldest_timestamp_to_render < oldest_rendered_timestamp {
                    buffer.clear();
                }
            }
        }
        *self.oldest_rendered_timestamp.borrow_mut() = oldest_timestamp_to_render;

        // Holding a buffer ref should be fine even though the message type may access it as it
        // cannot be accessed mutably, which would panic.
        // It is however important not to hold onto the mutable state reference, as it is very likely
        // the message type will access the state while rendering
        buffer.disable_print_hooks();
        for msg in messages {
            self.print_msg(msg, false);
            if let Some(last_read_id) = &*last_read_id {
                if &msg.id(&mut self.state.borrow_mut()) == last_read_id {
                    buffer.mark_read();
                }
            }
        }
        buffer.enable_print_hooks();
        buffer.clear_hotlist();
    }

    pub fn update_message<F>(&self, id: &I, f: F)
    where
        F: FnOnce(&mut M),
    {
        let mut state = self.state.borrow_mut();
        if let Some(msg) = self
            .messages
            .borrow_mut()
            .iter_mut()
            .find(|msg| &msg.id(&mut state) == id)
        {
            f(msg);
        }
    }

    pub fn get_nth_message(&self, index: usize) -> Option<M> {
        self.messages.borrow().iter().nth(index).cloned()
    }

    pub fn nth_oldest_message(&self, index: usize) -> Option<M> {
        self.messages.borrow().iter().rev().nth(index).cloned()
    }

    pub fn remove_msg(&self, id: &I) {
        {
            let mut state = self.state.borrow_mut();
            let index = self
                .messages
                .borrow()
                .iter()
                .position(|it| &it.id(&mut state) == id);
            if let Some(index) = index {
                self.messages.borrow_mut().remove(index);
            }
        }
        self.redraw_buffer();
    }
}

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

pub struct History {
    first: Option<Rc<RefCell<Node>>>,
    last: Option<Rc<RefCell<Node>>>,
    current: Option<Rc<RefCell<Node>>>, // cursor for navigation
    len: usize,

    settings: HistorySettings,
}

pub struct HistorySettings {
    pub is_capped: bool,
    pub capacity: usize,
}

struct Node {
    data: String,
    next: Option<Rc<RefCell<Node>>>,
    prev: Option<Weak<RefCell<Node>>>,
}

impl History {
    pub fn new(new_setting: HistorySettings) -> Self {
        Self {
            first: None,
            last: None,
            current: None,
            len: 0,
            settings: new_setting,
        }
    }

    pub fn set_setting(&mut self, new_setting: HistorySettings) {
        self.settings = new_setting;
        if self.settings.is_capped && self.len > self.settings.capacity {
            let extra = self.len - self.settings.capacity;
            for _ in 0..extra {
                self.remove_first();
            }
        }
    }

    fn remove_first(&mut self) {
        if let Some(first_node) = self.first.take() {
            let next = first_node.borrow_mut().next.take();
            if let Some(next_node) = &next {
                next_node.borrow_mut().prev = None;
            }

            self.first = next;
            self.len -= 1;

            if self.len == 0 {
                self.last = None;
                self.current = None;
                self.current = None;
            }
        }
    }

    pub fn add_to_history(&mut self, command: String) {
        if command.trim().is_empty() {
            return;
        }

        //dont add repeting dupes
        if self.last.is_some() && self.last.as_ref().unwrap().borrow().data == command {
            return;
        }

        // list is at capasite
        if self.settings.is_capped && self.len >= self.settings.capacity {
            self.remove_first();
        }

        let new_node = Rc::new(RefCell::new(Node {
            data: command,
            next: None,
            prev: None,
        }));
        match self.last.take() {
            Some(last_node) => {
                last_node.borrow_mut().next = Some(Rc::clone(&new_node));
                new_node.borrow_mut().prev = Some(Rc::downgrade(&last_node));
                self.last = Some(new_node);
            }
            None => {
                // List is empty
                self.first = Some(Rc::clone(&new_node));
                self.last = Some(Rc::clone(&new_node));
            }
        }
        self.len += 1;

        self.current = None;
    }

    pub fn previous(&mut self) -> Option<String> {
        if self.current.is_none() {
            self.current = self.last.clone();
            return self.current.as_ref().map(|n| n.borrow().data.clone());
        }

        let new_current = {
            let current = self.current.as_ref().unwrap();
            current.borrow().prev.as_ref().and_then(|w| w.upgrade())
        };
        if let Some(prev_node) = new_current {
            self.current = Some(Rc::clone(&prev_node));
            return Some(prev_node.borrow().data.clone());
        }
        None
    }

    pub fn next(&mut self) -> Option<String> {
        let new_current = {
            let current = self.current.as_ref()?;
            current.borrow().next.clone()
        };

        if let Some(next_node) = new_current {
            self.current = Some(Rc::clone(&next_node));
            return Some(next_node.borrow().data.clone());
        }

        self.current = None;
        None
    }

    pub fn set_to_begining(&mut self){
        self.current = None;
    }
}

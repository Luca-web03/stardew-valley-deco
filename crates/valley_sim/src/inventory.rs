use serde::{Deserialize, Serialize};

use crate::item::{Item, MAX_STACK};

pub const TOOLBAR: usize = 12;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Inventory {
    pub slots: Vec<Option<Item>>,
}

impl Inventory {
    pub fn new(size: usize) -> Self {
        Self { slots: vec![None; size] }
    }

    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    pub fn get(&self, i: usize) -> Option<&Item> {
        self.slots.get(i).and_then(|s| s.as_ref())
    }

    pub fn get_mut(&mut self, i: usize) -> Option<&mut Item> {
        self.slots.get_mut(i).and_then(|s| s.as_mut())
    }

    pub fn take(&mut self, i: usize) -> Option<Item> {
        self.slots.get_mut(i).and_then(|s| s.take())
    }

    /// Add an item, stacking first, then into empty slots. Returns whatever
    /// didn't fit.
    pub fn add(&mut self, mut item: Item) -> Option<Item> {
        for slot in self.slots.iter_mut().flatten() {
            if slot.can_stack_with(&item) {
                let space = MAX_STACK - slot.stack();
                if space > 0 {
                    let n = space.min(item.stack());
                    slot.set_stack(slot.stack() + n);
                    item.set_stack(item.stack() - n);
                    if item.stack() <= 0 {
                        return None;
                    }
                }
            }
        }
        for slot in self.slots.iter_mut() {
            if slot.is_none() {
                *slot = Some(item);
                return None;
            }
        }
        Some(item)
    }

    pub fn can_fit(&self, item: &Item) -> bool {
        self.slots.iter().any(|s| match s {
            None => true,
            Some(i) => i.can_stack_with(item) && i.stack() + item.stack() <= MAX_STACK,
        })
    }

    /// Remove `n` from the stack in slot `i`; clears the slot when empty.
    pub fn reduce(&mut self, i: usize, n: i32) {
        if let Some(Some(item)) = self.slots.get_mut(i) {
            let left = item.stack() - n;
            if left <= 0 || item.tool().is_some() {
                self.slots[i] = None;
            } else {
                item.set_stack(left);
            }
        }
    }

    pub fn count(&self, object_id: &str) -> i32 {
        self.slots.iter().flatten().filter(|i| i.object_id() == Some(object_id)).map(|i| i.stack()).sum()
    }

    /// Remove up to `n` of an object id across stacks; returns removed count.
    pub fn remove_object(&mut self, object_id: &str, mut n: i32) -> i32 {
        let mut removed = 0;
        for i in 0..self.slots.len() {
            if n <= 0 {
                break;
            }
            let Some(item) = self.slots[i].as_ref() else { continue };
            if item.object_id() != Some(object_id) {
                continue;
            }
            let take = item.stack().min(n);
            self.reduce(i, take);
            n -= take;
            removed += take;
        }
        removed
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        if a < self.slots.len() && b < self.slots.len() {
            self.slots.swap(a, b);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stacking() {
        let mut inv = Inventory::new(2);
        assert!(inv.add(Item::object("a", 990)).is_none());
        assert!(inv.add(Item::object("a", 20)).is_none());
        assert_eq!(inv.get(0).unwrap().stack(), 999);
        assert_eq!(inv.get(1).unwrap().stack(), 11);
        let left = inv.add(Item::object("b", 1));
        assert!(left.is_some());
        assert_eq!(inv.count("a"), 1010);
        assert_eq!(inv.remove_object("a", 1000), 1000);
        assert_eq!(inv.count("a"), 10);
    }
}

use std::any::TypeId;

use rustc_hash::FxHashMap;

use super::{event_future::EventPromise, EventKey};
use crate::{Event, EventData, Id};

#[derive(Clone)]
pub(crate) struct EventPromiseStore {
    promises: FxHashMap<AwaitKey, EventPromise>,
    promises_with_source: FxHashMap<AwaitKey, FxHashMap<Id, EventPromise>>,
}

impl EventPromiseStore {
    pub fn new() -> Self {
        Self {
            promises: FxHashMap::default(),
            promises_with_source: FxHashMap::default(),
        }
    }

    pub fn insert<T: EventData>(
        &mut self,
        dst: Id,
        src: Option<Id>,
        event_key: Option<EventKey>,
        promise: EventPromise,
    ) -> Result<(), String> {
        let key = AwaitKey::new::<T>(dst, event_key);

        // check that promise with such key (with or without source) doesn't exist yet
        if self.promises.contains_key(&key) {
            return Err(format!("Event promise for key {:?} already exists", key));
        }

        // store promise
        if let Some(src) = src {
            if let Some(promises) = self.promises_with_source.get(&key) {
                // check that promise with such key and source doesn't exist yet
                if promises.contains_key(&src) {
                    return Err(format!(
                        "Event promise for key {:?} with source {} already exists",
                        key, src
                    ));
                }
            }
            self.promises_with_source.entry(key).or_default().insert(src, promise);
        } else {
            if let Some(promises) = self.promises_with_source.get(&key) {
                // check that promise with such key and some source doesn't exist yet
                if !promises.is_empty() {
                    return Err(format!(
                        "Event promise for key {:?} with source {} already exists",
                        key,
                        promises.keys().next().unwrap(),
                    ));
                }
            }
            self.promises.insert(key, promise);
        }
        Ok(())
    }

    pub fn remove<T: EventData>(
        &mut self,
        dst: Id,
        src: &Option<Id>,
        event_key: Option<EventKey>,
    ) -> Option<EventPromise> {
        let key = AwaitKey::new::<T>(dst, event_key);
        if let Some(src) = src {
            if let Some(promises) = self.promises_with_source.get_mut(&key) {
                promises.remove(src)
            } else {
                None
            }
        } else {
            self.promises.remove(&key)
        }
    }

    pub fn has_promise_for(&self, event: &Event, event_key: Option<EventKey>) -> bool {
        let key = AwaitKey::new_by_ref(event.dst, event.data.as_ref(), event_key);
        if self.promises.contains_key(&key) {
            return true;
        }
        if let Some(promises) = self.promises_with_source.get(&key) {
            return promises.contains_key(&event.src);
        }
        false
    }

    pub fn remove_promise_for(&mut self, event: &Event, event_key: Option<EventKey>) -> Option<EventPromise> {
        let key = AwaitKey::new_by_ref(event.dst, event.data.as_ref(), event_key);
        if let Some(promise) = self.promises.remove(&key) {
            return Some(promise);
        }
        if let Some(promises) = self.promises_with_source.get_mut(&key) {
            return promises.remove(&event.src);
        }
        None
    }

    pub fn drop_promises_by_dst(&mut self, dst: Id) -> u32 {
        let mut removed_count = 0;
        self.promises.retain(|key, promise| {
            if key.dst == dst {
                promise.drop_state();
                removed_count += 1;
                return false;
            }
            true
        });
        self.promises_with_source.retain(|key, promises| {
            if key.dst == dst {
                promises.iter_mut().for_each(|(_, promise)| {
                    promise.drop_state();
                    removed_count += 1;
                });
                return false;
            }
            true
        });
        removed_count
    }
}

#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy)]
struct AwaitKey {
    pub dst: Id,
    pub data_type: TypeId,
    event_key: Option<EventKey>,
}

impl AwaitKey {
    pub fn new<T: EventData>(dst: Id, event_key: Option<EventKey>) -> Self {
        Self {
            dst,
            data_type: TypeId::of::<T>(),
            event_key,
        }
    }

    pub fn new_by_ref(dst: Id, data: &dyn EventData, event_key: Option<EventKey>) -> Self {
        Self {
            dst,
            data_type: data.type_id(),
            event_key,
        }
    }
}

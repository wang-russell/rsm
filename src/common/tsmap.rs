//use std::collections::hash_map::{Iter,IterMut};
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use crate::common::{errcode,spin_lock::spin_lock_t};
//use indexmap::{map::Iter, map::IterMut, IndexMap as HashMap};
use std::collections::{hash_map::Iter,hash_map::IterMut,HashMap};
use std::{cmp::Eq, hash::Hash};
pub struct TsHashMap<K, V>
where
    K: std::cmp::Eq + std::hash::Hash,
{
    inner: HashMap<K, V>,
    lock:spin_lock_t,
}

fn get_item<'a, K: Eq + Hash, V>(m: &'a HashMap<K, V>, k: &K) -> Result<&'a V, errcode::RESULT> {
    match m.get(k) {
        None => Err(errcode::ERROR_NOT_FOUND),
        Some(v) => Ok(v),
    }
}

impl<K, V> TsHashMap<K, V>
where
    K: std::cmp::Eq + std::hash::Hash,
{
    pub fn new(capacity:usize) -> Self {
        let map = Self {
            inner: HashMap::with_capacity(capacity),
            lock:spin_lock_t::new(),
        };
        return map;
    }
    pub fn with_capacity(capacity:usize) -> Self {
        Self::new(capacity)
    }

    pub fn get(&self, k: &K) -> Option<&V> {
        self.lock.lock();
        let v= self.inner.get(k);
        self.lock.unlock();
        return v

    }
    pub fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        self.lock.lock();
        let v= self.inner.get_mut(k);
        self.lock.unlock();
        return v
    }
    pub fn contains_key(&self,k:&K)->bool {
        return self.is_exist(k)
    }
    pub fn is_exist(&self,k:&K)->bool {
    
        self.lock.lock();
        let v= self.inner.contains_key(k);
        self.lock.unlock();
        return v

    }

    pub fn insert(&mut self, k: K, v: V) -> errcode::RESULT {
        self.lock.lock();
           let v= match self.inner.insert(k, v) {
                None => errcode::RESULT_SUCCESS,
                Some(_) => errcode::ERROR_ALREADY_EXIST,
            };
            self.lock.unlock();
            return v

    }

    pub fn remove(&mut self, k: &K) -> Option<V> {
        self.lock.lock();
        let v= self.inner.remove(k);
        self.lock.unlock();
       return v
    }
    //根据Key值，返回一个键值的下标
    /*
    pub fn get_index_of(&self, k: &K) -> Option<usize> {
        match self.inner.lock() {
            Ok(l) => l.get_index_of(k),
            Err(_) => None,
        }
    } */
    //根据下标，返回相应的Key、Value
    /*
    pub fn get_index(&mut self, index: usize) -> Option<(&K, &V)> {
        match self.inner.lock() {
            Ok(l) => l.get_index(index),
            Err(_) => None,
        }
    }

    pub fn get_index_mut(&mut self, index: usize) -> Option<(&K, &mut V)> {
        match self.inner.lock() {
            Ok(l) => l.get_index_mut(index),
            Err(_) => None,
        }
    }
    */

    ///iter将进行加锁，需要用end_iter()进行解锁
    pub fn iter(&self) -> Iter<'_, K, V> {
        self.lock.lock();
        return self.inner.iter()
    }

    pub fn end_iter(&self) {
        self.lock.unlock();
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        self.lock.lock();
        return self.inner.iter_mut();
    }
    pub fn len(&self)->usize {
        self.lock.lock();
        let v= self.inner.len();
        self.lock.unlock();
       return v     
    }

    pub fn capacity(&self)->usize {
        self.lock.lock();
        let v= self.inner.capacity();
        self.lock.unlock();
       return v
    }

    pub fn clear(&mut self) {
        self.lock.lock();
        self.inner.clear();
        self.lock.unlock();
    }
}

#![no_std]

extern crate alloc;

use alloc::alloc::{alloc_zeroed, dealloc, handle_alloc_error, Layout};
use core::iter::FromIterator;
use core::iter::Iterator;
use core::marker::{Send, Sync};
use core::ops;
use core::ptr::{self, NonNull};
use core::{fmt, mem, slice};


#[cfg(test)]
use alloc::format;
#[cfg(test)]
use alloc::string::ToString;

pub struct Vec<T> {
    ptr: NonNull<T>,
    cap: usize,
    len: usize,
}

unsafe impl<T: Send> Send for Vec<T> {}
unsafe impl<T: Sync> Sync for Vec<T> {}

#[inline]
fn do_alloc<T>(cap: usize) -> Option<(NonNull<T>, usize)> {
    if cap == 0 {
        return None;
    }
    let cap = cap.next_power_of_two();
    let size = cap.checked_mul(mem::size_of::<T>())?;
    let layout = Layout::from_size_align(size, mem::align_of::<T>()).ok()?;
    let raw = unsafe { alloc_zeroed(layout) };
    if raw.is_null() {
        handle_alloc_error(layout);
    }
    Some((unsafe { NonNull::new_unchecked(raw as *mut T) }, cap))
}

#[inline]
fn do_dealloc<T>(ptr: NonNull<T>, cap: usize) {
    if cap == 0 {
        return;
    }
    let layout = Layout::array::<T>(cap).expect("invalid layout");
    unsafe { dealloc(ptr.as_ptr() as *mut u8, layout) };
}

impl<T> Vec<T> {
    pub fn new() -> Self {
        Vec { ptr: NonNull::dangling(), cap: 0, len: 0 }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn capacity(&self) -> usize {
        self.cap
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn reserve(&mut self, additional: usize) {
        if self.len + additional <= self.cap {
            return;
        }
        let new_cap = (self.len + additional).next_power_of_two();
        let (new_ptr, real_cap) = do_alloc::<T>(new_cap).expect("OOM");

        unsafe {
            if self.cap > 0 {
                ptr::copy_nonoverlapping(self.ptr.as_ptr(), new_ptr.as_ptr(), self.len);
            }
        }

        do_dealloc(self.ptr, self.cap);
        self.ptr = new_ptr;
        self.cap = real_cap;
    }

    pub fn push(&mut self, value: T) {
        if self.len == self.cap {
            self.reserve(1);
        }
        unsafe {
            ptr::write(self.ptr.as_ptr().add(self.len), value);
        }
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        unsafe {
            let slot = self.ptr.as_ptr().add(self.len);
            Some(ptr::read(slot))
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }
        unsafe { Some(&*self.ptr.as_ptr().add(index)) }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.len {
            return None;
        }
        unsafe { Some(&mut *self.ptr.as_ptr().add(index)) }
    }

    pub fn remove(&mut self, index: usize) -> T {
        assert!(index < self.len, "out of bounds");
        unsafe {
            let ret = ptr::read(self.ptr.as_ptr().add(index));
            ptr::copy(
                self.ptr.as_ptr().add(index + 1),
                self.ptr.as_ptr().add(index),
                self.len - index - 1,
            );
            self.len -= 1;
            ret
        }
    }

    pub fn clear(&mut self) {
        while let Some(_) = self.pop() {}
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> Drop for Vec<T> {
    fn drop(&mut self) {
        unsafe {
            for i in (0..self.len).rev() {
                ptr::drop_in_place(self.ptr.as_ptr().add(i) as *mut T);
            }
            do_dealloc(self.ptr, self.cap);
        }
    }
}

impl<T> Extend<T> for Vec<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for v in iter {
            self.push(v);
        }
    }
}

impl<T> FromIterator<T> for Vec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut v = Vec::new();
        v.extend(iter);
        v
    }
}

impl<T> IntoIterator for Vec<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        let raw = self.ptr;
        let (cap, len) = (self.cap, self.len);
        mem::forget(self);
        IntoIter { raw, cap, len, idx: 0 }
    }
}

pub struct IntoIter<T> {
    raw: NonNull<T>,
    cap: usize,
    len: usize,
    idx: usize,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if self.idx >= self.len {
            return None;
        }
        unsafe {
            let el = ptr::read(self.raw.as_ptr().add(self.idx));
            self.idx += 1;
            Some(el)
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.len - self.idx;
        (n, Some(n))
    }
}

impl<T> Drop for IntoIter<T> {
    fn drop(&mut self) {
        unsafe {
            for i in self.idx..self.len {
                ptr::drop_in_place(self.raw.as_ptr().add(i) as *mut T);
            }
            do_dealloc(self.raw, self.cap);
        }
    }
}

pub struct Iter<'a, T> {
    raw: NonNull<T>,
    len: usize,
    idx: usize,
    _marker: core::marker::PhantomData<&'a T>,
}

pub struct IterMut<'a, T> {
    raw: NonNull<T>,
    len: usize,
    idx: usize,
    _marker: core::marker::PhantomData<&'a mut T>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.len {
            return None;
        }
        unsafe {
            let el = &*self.raw.as_ptr().add(self.idx);
            self.idx += 1;
            Some(el)
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.len - self.idx;
        (n, Some(n))
    }
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.len {
            return None;
        }
        unsafe {
            let el = &mut *self.raw.as_ptr().add(self.idx);
            self.idx += 1;
            Some(el)
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.len - self.idx;
        (n, Some(n))
    }
}

impl<'a, T> IntoIterator for &'a Vec<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        Iter {
            raw: self.ptr,
            len: self.len,
            idx: 0,
            _marker: core::marker::PhantomData,
        }
    }
}

impl<'a, T> IntoIterator for &'a mut Vec<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        IterMut {
            raw: self.ptr,
            len: self.len,
            idx: 0,
            _marker: core::marker::PhantomData,
        }
    }
}


impl<T: fmt::Debug> fmt::Debug for Vec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Vec {{ len: {}, cap: {}, data: {:?} }}", self.len, self.cap, self.as_slice())
    }
}

impl<T: fmt::Debug> fmt::Display for Vec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (i, v) in self.as_slice().iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{:?}", v)?;
        }
        write!(f, "]")
    }
}

impl<T> Default for Vec<T> {
    fn default() -> Self { Self::new() }
}

impl<T, I: slice::SliceIndex<[T]>> ops::Index<I> for Vec<T> {
    type Output = I::Output;
    fn index(&self, idx: I) -> &Self::Output {
        self.as_slice().index(idx)
    }
}
impl<T, I: slice::SliceIndex<[T]>> ops::IndexMut<I> for Vec<T> {
    fn index_mut(&mut self, idx: I) -> &mut Self::Output {
        self.as_mut_slice().index_mut(idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basics_push_pop() {
        let mut v = Vec::<i32>::new();
        assert!(v.is_empty());
        v.push(1);
        v.push(2);
        v.push(3);
        assert_eq!(v.len(), 3);
        assert_eq!(v.capacity(), 4);

        assert_eq!(v.pop(), Some(3));
        assert_eq!(v.pop(), Some(2));
        assert_eq!(v.pop(), Some(1));
        assert_eq!(v.pop(), None);
        assert!(v.is_empty());
    }

    #[test]
    fn grow_over_capacity() {
        let mut v = Vec::new();
        for i in 0..100 {
            v.push(i);
        }
        assert_eq!(v.len(), 100);
        assert!(v.capacity() >= 128);
    }

    #[test]
    fn get_and_index() {
        let mut v = Vec::new();
        v.push(10);
        v.push(20);
        v.push(30);
        assert_eq!(v.get(0), Some(&10));
        assert_eq!(v.get(5), None);
        assert_eq!(v[1], 20);
        assert_eq!(v[2], 20 + 10);
    }

    #[test]
    fn get_mut_mutation() {
        let mut v = Vec::new();
        v.push("hello".to_string());
        v.push("world".to_string());
        if let Some(s) = v.get_mut(1) {
            s.push('!');
        }
        assert_eq!(v[1], "world!");
    }

    #[test]
    fn remove_and_shift() {
        let mut v = Vec::new();
        v.push(1);
        v.push(2);
        v.push(3);
        v.push(4);

        assert_eq!(v.remove(1), 2);
        assert_eq!(v.len(), 3);
        assert_eq!(v[0], 1);
        assert_eq!(v[1], 3);
        assert_eq!(v[2], 4);
    }

    #[test]
    fn from_iter_and_into_iter() {
        let v: Vec<i32> = (0..5).collect();
        assert_eq!(v.len(), 5);
        assert_eq!(v[4], 4);

        let mut got = Vec::new();
        for x in v {
            got.push(x * 2);
        }
        assert_eq!(got.as_slice(), &[0, 2, 4, 6, 8]);
    }

    #[test]
    fn display_and_debug() {
        let v = Vec::from_iter([1, 2, 3]);
        assert_eq!(format!("{v}"), "[1, 2, 3]");
        let dbg = format!("{v:?}");
        assert!(dbg.contains("len: 3"));
    }

    #[test]
    fn clear_reuse() {
        let mut v = Vec::new();
        for i in 0..50 {
            v.push(i);
        }
        assert!(v.capacity() >= 64);
        v.clear();
        assert_eq!(v.len(), 0);
        assert!(v.capacity() >= 64);
        v.push(999);
        assert_eq!(v[0], 999);
    }

    #[test]
    fn drop_count() {
        use alloc::sync::Arc;
        use core::sync::atomic::{AtomicUsize, Ordering};

        static DROPS: AtomicUsize = AtomicUsize::new(0);
        #[allow(dead_code)]
        struct Counter(Arc<()>);

        impl Drop for Counter {
            fn drop(&mut self) {
                DROPS.fetch_add(1, Ordering::SeqCst);
            }
        }

        {
            let _v: Vec<Counter> = (0..100).map(|_| Counter(Arc::new(()))).collect();
            assert_eq!(DROPS.load(Ordering::SeqCst), 0);
        }
        assert_eq!(DROPS.load(Ordering::SeqCst), 100);
    }
}

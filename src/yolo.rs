use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    rc::Rc,
};

#[derive(Debug)]
pub struct YoloCell<T>(UnsafeCell<T>);

impl<T> YoloCell<T> {
    pub fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.0.get() }
    }

    pub fn ptr(&self) -> *mut T {
        self.0.get()
    }

    pub const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    pub fn into_inner(self) -> T {
        self.0.into_inner()
    }
}

impl<T> Deref for YoloCell<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.get() }
    }
}

impl<T> DerefMut for YoloCell<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0.get() }
    }
}

#[derive(Debug)]
pub struct YoloRc<T>(Rc<YoloCell<T>>);

impl<T> Clone for YoloRc<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> YoloRc<T> {
    pub fn new(value: T) -> Self {
        Self(Rc::new(YoloCell(UnsafeCell::new(value))))
    }
}

impl<T> Deref for YoloRc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for YoloRc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.get_mut()
    }
}

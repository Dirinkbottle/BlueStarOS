use core::cell::{RefCell};



//确保在单核环境中的数据安全共享访问
pub struct UPSafeCell<T>{
    inner:RefCell<T>
}

unsafe impl<T> Sync  for UPSafeCell<T>{}

impl<T> UPSafeCell<T>{
    pub const fn new(value:T)->Self{
        UPSafeCell{
            inner:RefCell::new(value)
        }
    }

    pub fn lock(&self)->core::cell::RefMut<'_,T>{
        self.inner.borrow_mut()
    }
}
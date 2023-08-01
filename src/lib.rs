#![allow(dead_code)]
struct Internal<T> {
    strong_count: usize,
    weak_count: usize,
    pointee: T
}

impl<T> Internal<T> {
    fn new(pointee: T) -> Self {
        Self { strong_count: 1, weak_count:0, pointee }
    }

    unsafe fn decrementweak(&mut self) {
        // eprintln!("decrement weak");
        self.weak_count -= 1;
        if self.strong_count == 0 && self.weak_count == 0 { unsafe {
            std::ptr::drop_in_place(&mut self.strong_count as *mut usize);
            std::ptr::drop_in_place(&mut self.weak_count as *mut usize);
        } }
    }

    unsafe fn incrementweak(&mut self) {
        self.weak_count = self.weak_count.checked_add(1).unwrap();
    }

    unsafe fn decrementstrong(&mut self) {
        // eprintln!("decrement strong");
        self.strong_count -= 1;
        if self.strong_count == 0 { unsafe {
                std::ptr::drop_in_place(&mut self.pointee as *mut T);
        } }
        if self.strong_count == 0 && self.weak_count == 0 { unsafe { 
            std::ptr::drop_in_place(&mut self.strong_count as *mut usize);
            std::ptr::drop_in_place(&mut self.weak_count as *mut usize);
        } }
    }

    unsafe fn incrementstrong(&mut self) {
        self.strong_count = self.strong_count.checked_add(1).unwrap();
    }

}

pub struct Rc<T> {
    ptr: *mut Internal<T>
}

impl<T> Rc<T> {
    pub fn new(pointee: T) -> Self {
        Rc { ptr: Box::into_raw(Box::new(Internal::new(pointee))) }
    }

    pub fn downgrade(self) -> Weak<T> {
        unsafe {
            (*self.ptr).incrementweak();
            (*self.ptr).decrementstrong();
            std::mem::transmute(self)
        }
    }
    pub fn new_cyclic(f: impl Fn(Weak<T>) -> T) -> Rc<T> {
        let filler = unsafe { std::mem::zeroed::<T>() };
        // be careful to never read this, it would cause a segfault at best
        let output = Rc { ptr: Box::into_raw(Box::new(Internal { strong_count: 0, weak_count: 1, pointee: filler }))};
        // allocate space, part of which to be replaced with output of closure/fn ptr
        unsafe { 
            core::ptr::write_volatile(output.ptr, Internal { strong_count: 1, weak_count: 1, pointee: f(Weak { ptr: output.ptr.clone() } ) } );
            // overwrites the filler data without reading it
        }
        output
    }
}


impl<T> core::ops::Deref for Rc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe{&(*self.ptr).pointee}
    }
}

impl<T> Clone for Rc<T> {
    fn clone(&self) -> Self {
        unsafe {
            (*self.ptr).incrementstrong();
        }
        Rc { ptr: self.ptr.clone() }
    }
}

impl<T> Drop for Rc<T> {
    fn drop(&mut self) {
        unsafe { (*self.ptr).decrementstrong(); }
    }
}

pub struct Weak<T> {
    ptr: *mut Internal<T>
}

impl<T> Weak<T> {
    pub fn upgrade(self) -> Option<Rc<T>> {
        if unsafe{(*self.ptr).strong_count} != 0 { // check if the pointer is dangling
            unsafe {
                (*self.ptr).incrementstrong(); // change counts so they stay in sync
                (*self.ptr).decrementweak();
                Some(std::mem::transmute(self)) // transmute since rc and weak are just smart pointers
            }
        } else { // the pointer is dangling, so you can't use it, return none
            None
        }
    }
}

impl<T> Clone for Weak<T> {
    fn clone(&self) -> Self {
        unsafe {(*self.ptr).weak_count += 1;}
        Self { ptr: self.ptr }
    }
}

impl<T> Drop for Weak<T> {
    fn drop(&mut self) {
        #[allow(arithmetic_overflow)]
        unsafe { (*self.ptr).decrementweak(); }
    }
}

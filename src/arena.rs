#![allow(unused)]

#[cfg(target_family = "unix")]
mod unix {
    use std::{
        ffi::{c_int, c_long, c_void},
        ptr,
    };

    const PROT_NONE: c_int = 0;
    const PROT_READ: c_int = 1;
    const PROT_WRITE: c_int = 2;

    const MAP_SHARED: c_int = 0x01;
    const MAP_PRIVATE: c_int = 0x02;

    const MAP_FILE: c_int = 0x00;
    const MAP_FIXED: c_int = 0x10;

    #[cfg(target_os = "linux")]
    const MAP_ANONYMOUS: c_int = 0x20;
    #[cfg(target_os = "macos")]
    const MAP_ANONYMOUS: c_int = 0x1000;

    #[cfg(target_os = "linux")]
    const SC_PAGE_SIZE: c_int = 30;
    #[cfg(target_os = "macos")]
    const SC_PAGE_SIZE: c_int = 29;

    extern "C" {
        pub fn mmap(
            addr: *mut c_void,
            len: usize,
            prot: c_int,
            flags: c_int,
            fd: c_int,
            offset: isize,
        ) -> *mut c_void;

        pub fn mprotect(addr: *mut c_void, len: usize, prot: c_int) -> c_int;
        pub fn munmap(addr: *mut c_void, len: usize) -> c_int;

        pub fn sysconf(name: c_int) -> c_long;
    }

    pub unsafe fn vm_reserve(size_aligned: usize) -> *mut u8 {
        let reserved = mmap(
            ptr::null_mut(),
            size_aligned,
            PROT_NONE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        ) as *mut u8;

        if reserved as usize == !0 {
            panic!("vm_reserve: mmap failed");
        }

        reserved
    }

    pub unsafe fn vm_release(addr: *const u8, size_aligned: usize) {
        munmap(addr as _, size_aligned);
    }

    pub unsafe fn vm_commit(addr: *const u8, size_aligned: usize) {
        mprotect(addr as _, size_aligned, PROT_READ | PROT_WRITE);
    }

    pub unsafe fn vm_uncommit(addr: *const u8, size_aligned: usize) {
        mprotect(addr as _, size_aligned, PROT_NONE);
    }

    pub unsafe fn os_page_size() -> usize {
        return sysconf(SC_PAGE_SIZE) as usize;
    }
}

#[cfg(target_family = "windows")]
mod windows {
    use std::ffi::c_void;

    extern "C" {
        pub fn VirtualAlloc(
            lpAddress: *mut c_void,
            dwSize: usize,
            flAllocationType: u32,
            flProtect: u32,
        ) -> *mut c_void;

        pub fn VirtualFree(lpAddress: *mut c_void, dwSize: usize, dwFreeType: u32) -> bool;
    }

    pub unsafe fn vm_reserve(size_aligned: usize) -> *mut u8 {
        compile_error!("No vm_reserve on Windows yet")
    }

    pub unsafe fn vm_release(addr: *mut u8, size_aligned: usize) {
        compile_error!("No vm_release on Windows yet")
    }

    pub unsafe fn vm_commit(addr: *mut u8, size_aligned: usize) {
        compile_error!("No vm_commit on Windows yet")
    }

    pub unsafe fn vm_uncommit(addr: *mut u8, size_aligned: usize) {
        compile_error!("No vm_uncommit on Windows yet")
    }

    pub unsafe fn os_page_size() -> usize {
        compile_error!("No os_page_size on Windows yet")
    }
}

use std::{
    cell::Cell,
    fmt::{self, Debug},
    marker::PhantomData,
    mem,
    ops::{Index, IndexMut},
    slice,
};

#[cfg(target_family = "unix")]
use unix::*;

#[cfg(target_family = "windows")]
use windows::*;

const PAGES_PER_COMMIT: usize = 16;

pub const KIB: usize = 1024;
pub const MIB: usize = 1024 * KIB;
pub const GIB: usize = 1024 * MIB;
pub const TIB: usize = 1024 * GIB;

pub struct Arena {
    base_addr: *const u8,
    end_addr: *const u8,
    bump_addr: Cell<*const u8>,
}

impl Arena {
    pub fn new(addr_space_size: usize) -> Self {
        unsafe {
            let addr_space_size = ceil_align(addr_space_size, os_page_size());

            let base_addr = vm_reserve(addr_space_size).cast_const();
            let end_addr = base_addr.byte_add(addr_space_size);
            let bump_addr = Cell::new(base_addr);

            Arena {
                base_addr,
                end_addr,
                bump_addr,
            }
        }
    }

    #[inline]
    fn alloc_granularity() -> usize {
        unsafe { os_page_size() * PAGES_PER_COMMIT }
    }

    #[inline]
    pub fn alloc<T>(&self, value: T) -> &mut T {
        unsafe {
            let ptr = self.alloc_region(mem::size_of::<T>(), mem::align_of::<T>()) as *mut T;
            ptr.write(value);
            &mut *ptr
        }
    }

    #[inline]
    pub fn alloc_slice<T>(&self, size: usize) -> &mut [T] {
        unsafe {
            let ptr = self.alloc_region(size * mem::size_of::<T>(), mem::align_of::<T>());
            std::slice::from_raw_parts_mut(ptr as *mut T, size)
        }
    }

    unsafe fn alloc_region(&self, size: usize, align: usize) -> *mut u8 {
        let addr = ceil_align_ptr(self.bump_addr.get(), align);
        let next_bump_addr = addr.byte_add(size);

        if next_bump_addr > self.end_addr {
            panic!("Arena is out of memory");
        }

        // commit pages we don't have yet
        {
            let alloc_granularity = Self::alloc_granularity();
            let uncommitted_addr = ceil_align_ptr(self.bump_addr.get(), alloc_granularity);

            if next_bump_addr >= uncommitted_addr {
                let unaligned_commit_size = next_bump_addr.offset_from(uncommitted_addr) as usize;
                let commit_size = ceil_align(unaligned_commit_size, alloc_granularity);
                vm_commit(uncommitted_addr, commit_size);
            }
        }

        self.bump_addr.set(next_bump_addr);

        addr.cast_mut()
    }

    pub fn free_all(&mut self) {
        unsafe {
            let uncommitted_addr = ceil_align_ptr(self.bump_addr.get(), Self::alloc_granularity());
            let uncommit_size = uncommitted_addr.offset_from(self.base_addr) as usize;
            vm_uncommit(self.base_addr, uncommit_size);
        }

        self.bump_addr.set(self.base_addr);
    }
}

#[inline]
unsafe fn ceil_align_ptr<T>(ptr: *const T, to: usize) -> *const T {
    ceil_align(ptr as usize, to) as *const T
}

#[inline]
fn ceil_align(value: usize, to: usize) -> usize {
    let aligned = value + to - 1;
    aligned - aligned % to
}

// vector

/// A very rudimentary dynamic array backed by an arena.
pub struct Vector<T> {
    arena: Arena,
    _data: PhantomData<T>,
}

impl<T> Vector<T> {
    pub fn new(addr_space_size: usize) -> Self {
        Self {
            arena: Arena::new(addr_space_size),
            _data: PhantomData,
        }
    }

    pub fn add(&self, value: T) {
        self.arena.alloc(value);
    }

    pub fn get(&self, idx: usize) -> Option<&T> {
        if idx >= self.len() {
            None
        } else {
            unsafe {
                let ptr = self.arena.base_addr.byte_add(idx * mem::size_of::<T>());
                Some(&*(ptr as *const T))
            }
        }
    }

    pub fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
        if idx >= self.len() {
            None
        } else {
            unsafe {
                let ptr = self.arena.base_addr.byte_add(idx * mem::size_of::<T>());
                Some(&mut *(ptr as *mut T))
            }
        }
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe { &*slice::from_raw_parts_mut(self.arena.base_addr as _, self.len()) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { &mut *slice::from_raw_parts_mut(self.arena.base_addr as _, self.len()) }
    }

    pub fn iter(&self) -> impl Iterator<Item = &'_ T> {
        self.as_slice().into_iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &'_ mut T> {
        self.as_mut_slice().into_iter()
    }

    pub fn len(&self) -> usize {
        let bytes = unsafe { (self.arena.bump_addr.get()).byte_offset_from(self.arena.base_addr) };
        bytes as usize / mem::size_of::<T>()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&mut self) {
        self.arena.free_all();
    }
}

impl<T> Index<usize> for Vector<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        if let Some(value) = self.get(index) {
            value
        } else {
            panic!("Index out of bounds: {index} >= {}", self.len());
        }
    }
}

impl<T> IndexMut<usize> for Vector<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let len = self.len();

        if index >= len {
            panic!("Index out of bounds: {index} >= {len}");
        } else {
            unsafe {
                let ptr = self.arena.base_addr.byte_add(index * mem::size_of::<T>());
                &mut *(ptr as *mut T)
            }
        }
    }
}

impl<T: Debug> fmt::Debug for Vector<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}
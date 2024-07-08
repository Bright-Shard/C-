#![allow(unused)]

#[cfg(any(target_os = "linux", target_os = "macos"))]
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

    pub unsafe fn vm_release(addr: *mut u8, size_aligned: usize) {
        munmap(addr as _, size_aligned);
    }

    pub unsafe fn vm_commit(addr: *mut u8, size_aligned: usize) {
        mprotect(addr as _, size_aligned, PROT_READ | PROT_WRITE);
    }

    pub unsafe fn vm_uncommit(addr: *mut u8, size_aligned: usize) {
        mprotect(addr as _, size_aligned, PROT_NONE);
    }

    pub unsafe fn os_page_size() -> usize {
        sysconf(SC_PAGE_SIZE) as usize
    }
}

#[cfg(target_family = "windows")]
mod windows {
    use std::{
        ffi::{c_int, c_void},
        ptr,
    };

    const MEM_COMMIT: u32 = 0x00001000;
    const MEM_RESERVE: u32 = 0x00002000;

    const MEM_DECOMMIT: u32 = 0x00004000;
    const MEM_RELEASE: u32 = 0x00008000;

    const PAGE_NOACCESS: u32 = 0x01;
    const PAGE_READWRITE: u32 = 0x04;

    #[repr(C)]
    #[derive(Clone, Copy)]
    #[allow(non_snake_case)]
    pub struct DummyStructW {
        pub wProcessorArchitecture: u16,
        pub wReserved: u16,
    }

    #[repr(C)]
    #[allow(non_snake_case)]
    pub union DummySystemInfoUnion {
        pub dwOemId: u32,
        pub dummy: DummyStructW,
    }

    #[repr(C)]
    #[allow(non_snake_case)]
    pub struct SystemInfo {
        pub dummy: DummySystemInfoUnion,
        pub dwPageSize: u32,
        pub lpMinimumApplicationAddress: *const c_void,
        pub lpMaximumApplicationAddress: *const c_void,
        pub dwActiveProcessorMask: *const u32,
        pub dwNumberOfProcessors: u32,
        pub dwProcessorType: u32,
        pub dwAllocationGranularity: u32,
        pub wProcessorLevel: u16,
        pub wProcessorRevision: u16,
    }

    extern "C" {
        pub fn VirtualAlloc(
            lpAddress: *mut c_void,
            dwSize: usize,
            flAllocationType: u32,
            flProtect: u32,
        ) -> *mut c_void;

        pub fn VirtualFree(lpAddress: *mut c_void, dwSize: usize, dwFreeType: u32) -> bool;

        pub fn GetSystemInfo(lpSystemInfo: &mut SystemInfo);
    }

    pub unsafe fn vm_reserve(size_aligned: usize) -> *mut u8 {
        VirtualAlloc(ptr::null_mut(), size_aligned, MEM_RESERVE, PAGE_NOACCESS) as _
    }

    pub unsafe fn vm_release(addr: *mut u8, size_aligned: usize) {
        VirtualFree(addr as _, size_aligned, MEM_RELEASE);
    }

    pub unsafe fn vm_commit(addr: *mut u8, size_aligned: usize) {
        VirtualAlloc(addr as _, size_aligned, MEM_COMMIT, PAGE_READWRITE);
    }

    pub unsafe fn vm_uncommit(addr: *mut u8, size_aligned: usize) {
        VirtualFree(addr as _, size_aligned, MEM_DECOMMIT);
    }

    pub unsafe fn os_page_size() -> usize {
        let mut system_info = SystemInfo {
            dummy: DummySystemInfoUnion { dwOemId: 0 },
            dwPageSize: 0,
            lpMinimumApplicationAddress: ptr::null(),
            lpMaximumApplicationAddress: ptr::null(),
            dwActiveProcessorMask: ptr::null(),
            dwNumberOfProcessors: 0,
            dwProcessorType: 0,
            dwAllocationGranularity: 0,
            wProcessorLevel: 0,
            wProcessorRevision: 0,
        };

        GetSystemInfo(&mut system_info);
        system_info.dwPageSize as usize
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_family = "windows")))]
compile_error!("Operating system not supported");

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
    base_addr: *mut u8,
    end_addr: *mut u8,
    uncommitted_addr: Cell<*mut u8>,
    bump_addr: Cell<*mut u8>,
}

impl Arena {
    pub fn new(addr_space_size: usize) -> Self {
        unsafe {
            let addr_space_size = ceil_align(addr_space_size, os_page_size());

            let base_addr = vm_reserve(addr_space_size);
            let end_addr = base_addr.byte_add(addr_space_size);
            let uncommitted_addr = Cell::new(base_addr);
            let bump_addr = Cell::new(base_addr);

            Arena {
                base_addr,
                end_addr,
                uncommitted_addr,
                bump_addr,
            }
        }
    }

    #[inline]
    fn alloc_granularity() -> usize {
        unsafe { os_page_size() * PAGES_PER_COMMIT }
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn alloc<T>(&self, value: T) -> &mut T {
        unsafe {
            let ptr = self.alloc_region(mem::size_of::<T>(), mem::align_of::<T>()) as *mut T;
            ptr.write(value);
            &mut *ptr
        }
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn alloc_slice<T>(&self, size: usize) -> &mut [T] {
        unsafe {
            let ptr = self.alloc_region(size * mem::size_of::<T>(), mem::align_of::<T>());
            std::slice::from_raw_parts_mut(ptr as *mut T, size)
        }
    }

    unsafe fn alloc_region(&self, size: usize, align: usize) -> *mut u8 {
        let addr = ceil_align_ptr(self.bump_addr.get(), align);
        let next_bump_addr = addr.byte_add(size);

        // if next_bump_addr > self.end_addr {
        //     panic!("Arena is out of memory");
        // }

        // commit pages we don't have yet
        if next_bump_addr >= self.uncommitted_addr.get() {
            let alloc_granularity = Self::alloc_granularity();
            let uncommit_end_addr = ceil_align_ptr(next_bump_addr, alloc_granularity);
            let commit_size = uncommit_end_addr.offset_from(self.uncommitted_addr.get()) as usize;
            vm_commit(self.uncommitted_addr.get(), commit_size);
            self.uncommitted_addr.set(uncommit_end_addr);
        }

        self.bump_addr.set(next_bump_addr);

        addr
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

impl Drop for Arena {
    fn drop(&mut self) {
        unsafe {
            vm_release(self.base_addr, self.end_addr.offset_from(self.base_addr) as usize);
        }
    }
}

#[inline]
unsafe fn ceil_align_ptr<T>(ptr: *mut T, to: usize) -> *mut T {
    ceil_align(ptr as usize, to) as *mut T
}

/// Ceil-aligns the value. Assumes a power of 2.
#[inline]
fn ceil_align(value: usize, to: usize) -> usize {
    (value as isize + (-(value as isize) & (to as isize - 1))) as usize
}

// vector

/// A very rudimentary dynamic array backed by an arena.
pub struct ArenaVec<T> {
    arena: Arena,
    _data: PhantomData<T>,
}

impl<T> ArenaVec<T> {
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
        self.as_slice().iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &'_ mut T> {
        self.as_mut_slice().iter_mut()
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

impl<T> Index<usize> for ArenaVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        if let Some(value) = self.get(index) {
            value
        } else {
            panic!("Index out of bounds: {index} >= {}", self.len());
        }
    }
}

impl<T> IndexMut<usize> for ArenaVec<T> {
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

impl<T: Debug> fmt::Debug for ArenaVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

#[cfg(test)]
mod tests_ceil_align {
    use crate::arena::ceil_align;

    #[test]
    fn correct_8() {
        assert_eq!(ceil_align(0, 8), 0);
        assert_eq!(ceil_align(1, 8), 8);
        assert_eq!(ceil_align(2, 8), 8);
        assert_eq!(ceil_align(3, 8), 8);
        assert_eq!(ceil_align(4, 8), 8);
        assert_eq!(ceil_align(5, 8), 8);
        assert_eq!(ceil_align(6, 8), 8);
        assert_eq!(ceil_align(7, 8), 8);
        assert_eq!(ceil_align(8, 8), 8);
        assert_eq!(ceil_align(9, 8), 16);
    }

    #[test]
    fn correct_16() {
        assert_eq!(ceil_align(0, 16), 0);
        assert_eq!(ceil_align(1, 16), 16);
        assert_eq!(ceil_align(2, 16), 16);
        assert_eq!(ceil_align(3, 16), 16);
        assert_eq!(ceil_align(4, 16), 16);
        assert_eq!(ceil_align(5, 16), 16);
        assert_eq!(ceil_align(15, 16), 16);
        assert_eq!(ceil_align(16, 16), 16);
        assert_eq!(ceil_align(17, 16), 32);
        assert_eq!(ceil_align(18, 16), 32);
        assert_eq!(ceil_align(19, 16), 32);
    }
}

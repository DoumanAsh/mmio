//! Memory mapped IO

#![no_std]
#![warn(missing_docs)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::style))]

use core::{fmt, ptr, marker};

///Memory mapped raw pointer
pub struct RawPtr<'a, T> {
    ///Pointer
    pub ptr: *mut T,
    _lifetime: marker::PhantomData<&'a mut T>,
}

#[repr(transparent)]
///Memory mapped IO
pub struct MemoryMap<T> {
    ptr: *mut T
}

impl<T> MemoryMap<T> {
    #[inline]
    ///Reads data
    pub fn read(&self) -> T {
        unsafe {
            ptr::read_volatile(self.ptr)
        }
    }

    #[inline]
    ///Writes data
    pub fn write(&mut self, val: T) {
        unsafe {
            ptr::write_volatile(self.ptr, val)
        }
    }

    #[inline]
    ///Gives callback to accept value to return modified value to write.
    pub fn read_and_write<F: FnOnce(T) -> T>(&mut self, cb: F) {
        let new = cb(self.read());
        self.write(new);
    }

    #[inline]
    #[allow(clippy::needless_lifetimes)]
    ///Access raw pointer
    ///
    ///Note that, ownership is not transferred
    pub fn as_ref<'a>(&'a mut self) -> RawPtr<'a, T> {
        RawPtr {
            ptr: self.ptr,
            _lifetime: marker::PhantomData
        }
    }

    #[allow(unused)]
    #[inline]
    ///Opens memory map.
    ///
    ///## Arguments
    ///
    ///- `offset` - Offset within memory to start.
    ///- `fd` - File description. -1 for anonymous.
    ///- `prot` - Memory protection. Specifies operations to be expected. At the very least must be `PROT_READ | PROT_WRITE`
    ///- `flags` - Specifies whether changes to the mapping are visible across forks. Must be `MAP_ANON` for anonymous.
    pub unsafe fn open_file_raw(offset: libc::off_t, fd: libc::c_int, prot: libc::c_int, flags: libc::c_int) -> Option<Self> {
        #[cfg(unix)]
        {
            use core::mem;

            let page_size = libc::sysconf(libc::_SC_PAGESIZE) as libc::off_t;
            let offset_mask = (page_size - 1);
            let page_mask = !0u32 as libc::off_t ^ offset_mask;

            let ptr = libc::mmap(ptr::null_mut(), mem::size_of::<T>(), prot, flags, fd, offset & page_mask);

            if ptr == libc::MAP_FAILED {
                return None;
            }

            Some(Self {
                ptr: unsafe {
                    (ptr as *mut u8).add(offset as usize & offset_mask as usize) as *mut _
                }
            })
        }

        #[cfg(not(unix))]
        None
    }

    ///Creates anonymous memory mapping
    pub fn anonymous() -> Option<Self> {
        #[cfg(unix)]
        unsafe {
            Self::open_file_raw(0, -1, libc::PROT_READ | libc::PROT_WRITE, libc::MAP_ANON | libc::MAP_SHARED)
        }

        #[cfg(not(unix))]
        None
    }

    #[allow(unused)]
    ///Creates memory mapping on `/dev/mem` which accesses physical memory
    ///
    ///## Arguments
    ///
    ///- `offset` - Offset within memory to start.
    ///
    ///Returns `None` on error, further details can be examined by checking last IO error.
    pub unsafe fn dev_mem(offset: libc::off_t) -> Option<Self> {
        #[cfg(unix)]
        {
            const DEV_MEM: [u8; 9] = *b"/dev/mem\0";
            let fd = libc::open(DEV_MEM.as_ptr() as _, libc::O_RDWR | libc::O_SYNC);
            if fd == -1 {
                return None;
            }

            let result = Self::open_file_raw(offset, fd, libc::PROT_READ | libc::PROT_WRITE, libc::MAP_SHARED);
            libc::close(fd);
            result
        }

        #[cfg(not(unix))]
        None
    }
}

impl<T> Drop for MemoryMap<T> {
    #[inline]
    fn drop(&mut self) {
        if self.ptr.is_null() {
            return;
        }

        #[cfg(unix)]
        {
            use core::mem;

            let page_size = unsafe {
                libc::sysconf(libc::_SC_PAGESIZE) as u32
            };
            let offset_mask = page_size - 1;
            let page_mask: u32 = !0u32 ^ offset_mask;
            let base_addr = (self.ptr as usize) & page_mask as usize;
            unsafe {
                libc::munmap(mem::transmute(base_addr), mem::size_of::<T>());
            }
        }
    }
}

impl<T> fmt::Pointer for MemoryMap<T> {
    #[inline(always)]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.ptr, fmt)
    }
}

impl<T> fmt::Debug for MemoryMap<T> {
    #[inline(always)]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.ptr, fmt)
    }
}

unsafe impl<T> Send for MemoryMap<T> {
}

unsafe impl<T> Sync for MemoryMap<T> {
}

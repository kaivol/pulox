use core::alloc::{GlobalAlloc, Layout};

use crate::bindings::{mxCalloc_800, mxFree_800, mxMalloc_800};

#[global_allocator]
static MEX_ALLOCATOR: MexAllocator = MexAllocator;

struct MexAllocator;

unsafe impl GlobalAlloc for MexAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        mxMalloc_800(layout.size() as _) as _
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        mxFree_800(ptr as _)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        mxCalloc_800(layout.size() as _, 1) as _
    }
}

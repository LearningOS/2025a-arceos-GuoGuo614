#![no_std]

use allocator::{BaseAllocator, ByteAllocator, PageAllocator};

/// Early memory allocator
/// Use it before formal bytes-allocator and pages-allocator can work!
/// This is a double-end memory range:
/// - Alloc bytes forward
/// - Alloc pages backward
///
/// [ bytes-used | avail-area | pages-used ]
/// |            | -->    <-- |            |
/// start       b_pos        p_pos       end
///
/// For bytes area, 'count' records number of allocations.
/// When it goes down to ZERO, free bytes-used area.
/// For pages area, it will never be freed!
///
pub struct EarlyAllocator<const SIZE: usize> {
    start: usize,
    end: usize,
    b_pos: usize,
    p_pos: usize,
}

impl<const SIZE: usize> EarlyAllocator<SIZE> {
    pub const fn new() -> Self {
        Self {
            start: 0,
            end: 0 + 64 * 1024 * 1024,
            b_pos: 0,
            p_pos: 0 + 64 * 1024 * 1024,
        }
    }
}

#[inline]
fn align_up(addr: usize, align: usize) -> Option<usize> {
    debug_assert!(align.is_power_of_two());
    let mask = align - 1;
    addr.checked_add(mask).map(|x| x & !mask)
}

fn align_down(addr: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    addr & !(align - 1)
}

impl<const SIZE: usize> BaseAllocator for EarlyAllocator<SIZE> {
    fn init(&mut self, start: usize, size: usize) {
        self.start = start;
        let cap = core::cmp::min(size, 64 * 1024 * 1024);
        self.end = start + cap;
        self.b_pos = start;
        self.p_pos = self.end;
    }

    fn add_memory(&mut self, _start: usize, _size: usize) -> allocator::AllocResult {
        todo!()
    }
}

impl<const SIZE: usize> ByteAllocator for EarlyAllocator<SIZE> {
    fn alloc(
        &mut self,
        layout: core::alloc::Layout,
    ) -> allocator::AllocResult<core::ptr::NonNull<u8>> {
        let align = layout.align();
        let size = layout.size();

        let start = align_up(self.b_pos, align).ok_or(allocator::AllocError::InvalidParam)?;
        let end = start.checked_add(size).ok_or(allocator::AllocError::InvalidParam)?;

        if end > self.p_pos {
            return Err(allocator::AllocError::MemoryOverlap);
        }

        self.b_pos = end;
        Ok(unsafe {
            core::ptr::NonNull::new_unchecked(start as *mut u8)
        })
    }

    fn dealloc(&mut self, pos: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
        // LIFO free: only rewind if this block is the most recently allocated one.
        let start = pos.as_ptr() as usize;
        let size = layout.size();
        if let Some(end) = start.checked_add(size) {
            if end == self.b_pos {
                self.b_pos = start;
            }
        }
    }

    fn total_bytes(&self) -> usize {
        self.p_pos - self.start
    }

    fn used_bytes(&self) -> usize {
        self.b_pos - self.start
    }

    fn available_bytes(&self) -> usize {
        self.total_bytes() - self.used_bytes()
    }
}

impl<const SIZE: usize> PageAllocator for EarlyAllocator<SIZE> {
    const PAGE_SIZE: usize = SIZE;

    fn alloc_pages(
        &mut self,
        num_pages: usize,
        align_pow2: usize,
    ) -> allocator::AllocResult<usize> {
        let size = num_pages
            .checked_mul(Self::PAGE_SIZE)
            .ok_or(allocator::AllocError::InvalidParam)?;
        let req_align = if align_pow2 >= usize::BITS as usize {
            return Err(allocator::AllocError::InvalidParam);
        } else {
            core::cmp::max(Self::PAGE_SIZE, 1usize << align_pow2)
        };
        let end = align_down(self.p_pos, req_align);
        let start = end - size;

        if start < self.b_pos {
            return Err(allocator::AllocError::MemoryOverlap);
        }

        self.p_pos = start;
        Ok(start)
    }

    fn dealloc_pages(&mut self, _pos: usize, _num_pages: usize) {
        // EarlyAllocator's pages are never freed; this is a no-op by design.
    }

    fn total_pages(&self) -> usize {
        (self.end - self.b_pos) / Self::PAGE_SIZE
    }

    fn used_pages(&self) -> usize {
        (self.end - self.p_pos) / Self::PAGE_SIZE
    }

    fn available_pages(&self) -> usize {
        self.total_pages() - self.used_pages()
    }
}
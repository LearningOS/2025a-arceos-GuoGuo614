use crate::ctypes;
use axerrno::LinuxError;
use core::ffi::{c_int, c_void};

use axhal::paging::MappingFlags;
use axhal::mem::{PAGE_SIZE_4K, VirtAddr};

const MAP_FIXED: i32 = 0x10;
const MAP_FIXED_NOREPLACE: i32 = 0x100000;
const MAP_ANON: i32 = 0x20;

#[inline]
fn align_up(x: usize, a: usize) -> usize { (x + a - 1) & !(a - 1) }

pub fn sys_mmap(
    addr: *mut c_void,
    length: usize,
    prot: i32,
    flags: i32,
    fd: c_int,
    _offset: isize,
) -> isize {
    if length == 0 {
        return -LinuxError::EINVAL.code() as _;
    }
    // 这里只实现匿名映射
    if (flags & MAP_ANON) == 0 || fd != -1 {
        return -LinuxError::ENOSYS.code() as _;
    }

    let len = align_up(length, PAGE_SIZE_4K);
    let va_req = addr as usize;

    // MAP_FIXED 时必须页对齐且非 0
    if (flags & MAP_FIXED) != 0 {
        if va_req == 0 || (va_req & (PAGE_SIZE_4K - 1)) != 0 {
            return -LinuxError::EINVAL.code() as _;
        }
    }

    let mut mflags = MappingFlags::from_bits_truncate(prot as usize) | MappingFlags::USER;

    let mut aspace = axtask::current().task_ext().aspace.lock();

    // 选择/检查目标起始地址
    let vaddr = if (flags & MAP_FIXED) != 0 {
        let v = VirtAddr::from(va_req);
        if (flags & MAP_FIXED_NOREPLACE) != 0 {
            // 若有重叠则报错（替换为你实际的重叠检测接口）
            if aspace.overlaps(v, len) { // TODO: 按实际 API 替换
                return -LinuxError::EEXIST.code() as _;
            }
        } else {
            // 替换语义：先拆掉重叠区（没有则忽略）
            let _ = aspace.unmap(v, len); // TODO: 按实际 API 替换/删除
        }
        v
    } else {
        if va_req != 0 {
            let v = VirtAddr::from(va_req);
            // 把 addr 当作 hint；冲突则找其它空闲区
            if !aspace.overlaps(v, len) { // TODO: 按实际 API 替换
                v
            } else {
                aspace
                    .find_free_area(len, PAGE_SIZE_4K) // TODO: 按实际 API 替换
                    .ok_or(LinuxError::ENOMEM)
                    .map_err(|e| -e.code() as isize)
                    .and_then(|v| return v.as_usize() as isize)
                    .err()
                    .unwrap_or_else(|| VirtAddr::from(va_req)) // 占位，避免编译错误
            }
        } else {
            // 未提供 hint：选择一段空闲区
            match aspace.find_free_area(len, PAGE_SIZE_4K) { // TODO: 按实际 API 替换
                Some(v) => v,
                None => return -LinuxError::ENOMEM.code() as _,
            }
        }
    };

    // 建立映射并分配物理页（立即分配）
    if let Err(e) = aspace.map_alloc(vaddr, len, mflags, true) {
        return -e.code() as isize;
    }

    // 清零映射区域
    unsafe { core::ptr::write_bytes(vaddr.as_usize() as *mut u8, 0, len); }

    vaddr.as_usize() as isize
}
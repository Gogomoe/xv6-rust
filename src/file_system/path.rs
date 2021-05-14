use alloc::string::String;
use core::ptr::null_mut;

use crate::file_system::inode::{ICACHE, INode};
use crate::file_system::ROOT_INO;
use crate::param::ROOT_DEV;
use crate::process::CPU_MANAGER;

pub const TYPE_DIR: u16 = 1;
pub const TYPE_FILE: u16 = 2;
pub const TYPE_DEVICE: u16 = 3;

pub struct FileStatus {
    pub dev: u32,
    pub ino: u32,
    pub types: u16,
    pub nlink: u16,
    pub size: u32,
}

// Copy the next path element from path into name.
// Return a pointer to the element following the copied one.
// The returned path has no leading slashes,
// so the caller can check *path=='\0' to see if the name is the last one.
// If no name to remove, return 0.
//
// Examples:
//   skipelem("a/bb/c", name) = "bb/c", setting name = "a"
//   skipelem("///a//bb", name) = "bb", setting name = "a"
//   skipelem("a", name) = "", setting name = "a"
//   skipelem("", name) = skipelem("////", name) = 0
fn split_path(path: &String) -> Option<(String, String)> {
    let mut name = String::new();
    let mut remain = String::new();

    let mut finish_name = false;
    for char in path.chars() {
        if char == '/' {
            if name.is_empty() {
                continue;
            } else if !finish_name {
                finish_name = true;
            } else if !remain.is_empty() {
                remain.push(char);
            }
        } else {
            if !finish_name {
                name.push(char);
            } else {
                remain.push(char);
            }
        }
    }

    if name.is_empty() {
        return None;
    }
    Some((name, remain))
}

// Look up and return the inode for a path name.
// If parent != 0, return the inode for the parent and copy the final
// path element into name, which must have room for DIRSIZ bytes.
// Must be called inside a transaction since it calls iput().
pub fn find_inode(path: &String) -> Option<&'static INode> {
    let mut ip: *const INode = if path.starts_with("/") {
        ICACHE.get(ROOT_DEV, ROOT_INO) as *const INode
    } else {
        let current_dir = CPU_MANAGER.my_proc().data().current_dir.unwrap();
        current_dir.dup() as *const INode
    };

    let mut next_level = split_path(path);
    while next_level.is_some() {
        let ip_ref = unsafe { ip.as_ref() }.unwrap();

        let (name, remain_path) = next_level.unwrap();
        let guard = ip_ref.lock();
        if ip_ref.data().types != TYPE_DIR {
            ip_ref.unlock(guard);
            ICACHE.put(ip_ref);
            return None;
        }
        let next_dir = ip_ref.dir_lookup(name.as_bytes(), null_mut());
        if next_dir.is_none() {
            ip_ref.unlock(guard);
            ICACHE.put(ip_ref);
            return None;
        }
        ip_ref.unlock(guard);
        ICACHE.put(ip_ref);

        ip = next_dir.unwrap();
        next_level = split_path(&remain_path);
    }

    return unsafe { ip.as_ref() };
}

pub fn find_inode_parent(path: &String) -> Option<(&INode, String)> {
    let mut ip: *const INode = if path.starts_with("/") {
        ICACHE.get(ROOT_DEV, ROOT_INO) as *const INode
    } else {
        let current_dir = CPU_MANAGER.my_proc().data().current_dir.unwrap();
        current_dir.dup() as *const INode
    };

    let mut next_level = split_path(path);
    if next_level.is_none() {
        return None;
    }

    while next_level.is_some() {
        let ip_ref = unsafe { ip.as_ref() }.unwrap();

        let (name, remain_path) = next_level.unwrap();
        let guard = ip_ref.lock();
        if ip_ref.data().types != TYPE_DIR {
            ip_ref.unlock(guard);
            ICACHE.put(ip_ref);
            return None;
        }
        if remain_path.is_empty() {
            // Stop one level early.
            ip_ref.unlock(guard);
            return Some((unsafe { ip.as_ref() }.unwrap(), name));
        }
        let next_dir = ip_ref.dir_lookup(name.as_bytes(), null_mut());
        if next_dir.is_none() {
            ip_ref.unlock(guard);
            ICACHE.put(ip_ref);
            return None;
        }
        ip_ref.unlock(guard);
        ICACHE.put(ip_ref);

        ip = next_dir.unwrap();
        next_level = split_path(&remain_path);
    }

    panic!("should not arrive here")
}
//! Capability-bound local reads. On Unix every component is opened relative to
//! an already opened directory descriptor with O_NOFOLLOW. Source content never
//! follows a replaced parent directory or a symlink outside approved roots.
use crate::*;
#[cfg(unix)]
mod native {
    use super::*;
    use std::{
        ffi::{CStr, CString},
        fs::File,
        io::{Read, Write},
        os::fd::{AsRawFd, FromRawFd, IntoRawFd},
        os::unix::fs::OpenOptionsExt,
        sync::atomic::{AtomicU64, Ordering},
    };
    pub struct Root {
        pub path: PathBuf,
        dir: File,
    }
    fn name(s: &str) -> Result<CString> {
        CString::new(s).map_err(|_| fail("TI_PROJECT", "invalid path component"))
    }
    fn open_at(parent: &File, child: &str, directory: bool) -> Result<File> {
        let flags = libc::O_RDONLY
            | libc::O_CLOEXEC
            | libc::O_NOFOLLOW
            | libc::O_NONBLOCK
            | if directory { libc::O_DIRECTORY } else { 0 };
        // SAFETY: parent is a live owned descriptor; the NUL-terminated name is
        // valid for the call; ownership of a successful new descriptor is unique.
        let fd = unsafe { libc::openat(parent.as_raw_fd(), name(child)?.as_ptr(), flags) };
        if fd < 0 {
            return Err(fail(
                "TI_PROJECT",
                "path is missing, inaccessible, or a symlink",
            ));
        }
        Ok(unsafe { File::from_raw_fd(fd) })
    }
    impl Root {
        pub fn new(path: PathBuf) -> Result<Self> {
            let dir = std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(&path)
                .map_err(|_| fail("TI_PROJECT", "cannot open authorized workspace root"))?;
            Ok(Self { path, dir })
        }
        fn parts(&self, path: &Path) -> Result<Vec<String>> {
            let relative = path
                .strip_prefix(&self.path)
                .map_err(|_| fail("TI_PROJECT", "path outside authorized root"))?;
            relative
                .components()
                .map(|c| match c {
                    std::path::Component::Normal(s) => s
                        .to_str()
                        .map(String::from)
                        .ok_or_else(|| fail("TI_PROJECT", "non-UTF-8 path")),
                    _ => Err(fail("TI_PROJECT", "invalid relative path")),
                })
                .collect()
        }
        fn open(&self, path: &Path, directory: bool) -> Result<File> {
            let parts = self.parts(path)?;
            let mut dir = self
                .dir
                .try_clone()
                .map_err(|_| fail("TI_PROJECT", "cannot clone workspace descriptor"))?;
            for (i, p) in parts.iter().enumerate() {
                dir = open_at(&dir, p, i + 1 < parts.len() || directory)?;
            }
            Ok(dir)
        }
        pub fn read(&self, path: &Path, max: usize) -> Result<Vec<u8>> {
            let file = self.open(path, false)?;
            if !file
                .metadata()
                .map_err(|_| fail("TI_PROJECT", "file metadata unavailable"))?
                .is_file()
            {
                return Err(fail("TI_PROJECT", "regular file required"));
            }
            let mut out = vec![];
            file.take(max as u64 + 1)
                .read_to_end(&mut out)
                .map_err(|_| fail("TI_PROJECT", "file read failed"))?;
            if out.len() > max {
                return Err(fail("TI_LIMIT", "file exceeds byte budget"));
            }
            Ok(out)
        }
        pub fn entries(&self, path: &Path) -> Result<Vec<(String, bool)>> {
            let dir = self.open(path, true)?;
            let parent = dir
                .try_clone()
                .map_err(|_| fail("TI_PROJECT", "cannot clone directory descriptor"))?;
            // fdopendir owns the descriptor on success; close explicitly on error.
            let fd = dir.into_raw_fd();
            let ptr = unsafe { libc::fdopendir(fd) };
            if ptr.is_null() {
                unsafe { libc::close(fd) };
                return Err(fail("TI_PROJECT", "cannot enumerate source directory"));
            }
            struct Entries(*mut libc::DIR);
            impl Drop for Entries {
                fn drop(&mut self) {
                    unsafe { libc::closedir(self.0) };
                }
            }
            let entries = Entries(ptr);
            let mut names = vec![];
            loop {
                #[cfg(target_os = "macos")]
                unsafe {
                    *libc::__error() = 0;
                }
                #[cfg(target_os = "linux")]
                unsafe {
                    *libc::__errno_location() = 0;
                }
                let entry = unsafe { libc::readdir(entries.0) };
                if entry.is_null() {
                    if std::io::Error::last_os_error().raw_os_error().unwrap_or(0) != 0 {
                        return Err(fail("TI_PROJECT", "directory enumeration failed"));
                    }
                    break;
                }
                let n = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }
                    .to_str()
                    .map_err(|_| fail("TI_PROJECT", "non-UTF-8 source path"))?
                    .to_owned();
                if n == "." || n == ".." {
                    continue;
                }
                if names.len() >= 4096 {
                    return Err(fail("TI_LIMIT", "directory entry budget"));
                }
                let f = open_at(&parent, &n, false)?;
                let meta = f
                    .metadata()
                    .map_err(|_| fail("TI_PROJECT", "entry metadata unavailable"))?;
                if !meta.is_dir() && !meta.is_file() {
                    return Err(fail(
                        "TI_PROJECT",
                        "source entries must be regular files or directories",
                    ));
                }
                names.push((n, meta.is_dir()));
            }
            names.sort();
            Ok(names)
        }
        pub fn atomic(&self, path: &Path, bytes: &[u8]) -> Result<()> {
            let parent = self.open(
                path.parent()
                    .ok_or_else(|| fail("TI_PROJECT", "lock parent missing"))?,
                true,
            )?;
            let target = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| fail("TI_PROJECT", "invalid lock name"))?;
            let target = name(target)?;
            // Do not replace symlinks or special files as lockfiles.
            let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
            let existing = unsafe {
                libc::fstatat(
                    parent.as_raw_fd(),
                    target.as_ptr(),
                    stat.as_mut_ptr(),
                    libc::AT_SYMLINK_NOFOLLOW,
                )
            };
            if existing == 0 {
                let stat = unsafe { stat.assume_init() };
                if stat.st_mode & libc::S_IFMT != libc::S_IFREG {
                    return Err(fail("TI_PROJECT", "lock target is not a regular file"));
                }
            }
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let temp = name(&format!(
                ".Ti.lock.{}.{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ))?;
            let fd = unsafe {
                libc::openat(
                    parent.as_raw_fd(),
                    temp.as_ptr(),
                    libc::O_WRONLY
                        | libc::O_CREAT
                        | libc::O_EXCL
                        | libc::O_NOFOLLOW
                        | libc::O_CLOEXEC,
                    0o600,
                )
            };
            if fd < 0 {
                return Err(fail("TI_PROJECT", "cannot create temporary lock"));
            }
            let mut file = unsafe { File::from_raw_fd(fd) };
            let result = (|| {
                file.write_all(bytes)
                    .and_then(|_| file.sync_all())
                    .map_err(|_| fail("TI_PROJECT", "lock write failed"))?;
                if unsafe {
                    libc::renameat(
                        parent.as_raw_fd(),
                        temp.as_ptr(),
                        parent.as_raw_fd(),
                        target.as_ptr(),
                    )
                } != 0
                {
                    return Err(fail("TI_PROJECT", "atomic lock replacement failed"));
                }
                parent
                    .sync_all()
                    .map_err(|_| fail("TI_PROJECT", "lock directory sync failed"))?;
                Ok(())
            })();
            if result.is_err() {
                unsafe { libc::unlinkat(parent.as_raw_fd(), temp.as_ptr(), 0) };
            }
            result
        }
    }
}
#[cfg(unix)]
pub use native::Root;
#[cfg(not(unix))]
pub struct Root {
    pub path: PathBuf,
}
#[cfg(not(unix))]
impl Root {
    pub fn new(_: PathBuf) -> Result<Self> {
        Err(fail(
            "TI_PROJECT",
            "safe filesystem loading is not implemented on this platform",
        ))
    }
    pub fn read(&self, _: &Path, _: usize) -> Result<Vec<u8>> {
        unreachable!()
    }
    pub fn entries(&self, _: &Path) -> Result<Vec<(String, bool)>> {
        unreachable!()
    }
    pub fn atomic(&self, _: &Path, _: &[u8]) -> Result<()> {
        unreachable!()
    }
}

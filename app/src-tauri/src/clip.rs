//! Copies a secret (a private key or a recovery key) to the clipboard privately: marked so Windows
//! keeps it out of clipboard history (Win+V) and the cloud clipboard, and cleared again after
//! `CLEAR_AFTER`, unless something else has been copied since.

use std::time::Duration;

use zeroize::Zeroizing;

pub const CLEAR_AFTER: Duration = Duration::from_secs(30);

#[tauri::command]
pub fn copy_private(text: String) -> Result<(), String> {
    let text = Zeroizing::new(text);
    let seq = imp::set(&text)?;
    std::thread::spawn(move || {
        std::thread::sleep(CLEAR_AFTER);
        imp::clear_if_unchanged(seq);
    });
    Ok(())
}

#[cfg(windows)]
mod imp {
    use std::ptr::null_mut;
    use std::time::Duration;

    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, GetClipboardSequenceNumber, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
    };
    use windows_sys::Win32::Foundation::GlobalFree;
    use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    use zeroize::Zeroizing;

    const CF_UNICODETEXT: u32 = 13;

    /// Another program may hold the clipboard for a moment: a few tries.
    fn open() -> bool {
        for _ in 0..10 {
            if unsafe { OpenClipboard(null_mut()) } != 0 {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }

    fn format(name: &str) -> u32 {
        let w: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        unsafe { RegisterClipboardFormatW(w.as_ptr()) }
    }

    /// Puts `bytes` on the open clipboard as `format`. The clipboard owns the memory afterwards.
    unsafe fn put(format: u32, bytes: &[u8]) -> bool {
        if format == 0 {
            return false;
        }
        let h = GlobalAlloc(GMEM_MOVEABLE, bytes.len());
        if h.is_null() {
            return false;
        }
        let p = GlobalLock(h) as *mut u8;
        if p.is_null() {
            GlobalFree(h);
            return false;
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
        GlobalUnlock(h);
        if SetClipboardData(format, h).is_null() {
            GlobalFree(h);
            return false;
        }
        true
    }

    /// Returns the clipboard's sequence number after the copy, to tell later whether it changed.
    pub fn set(text: &str) -> Result<u32, String> {
        if !open() {
            return Err("The clipboard is busy. Try again.".into());
        }
        let wide: Zeroizing<Vec<u8>> = Zeroizing::new(text.encode_utf16().chain(Some(0)).flat_map(u16::to_le_bytes).collect());
        let no = 0u32.to_le_bytes();
        // Windows' documented markers: clipboard monitors skip it, history and cloud sync leave it out.
        let ok = unsafe {
            EmptyClipboard() != 0
                && put(CF_UNICODETEXT, &wide)
                && put(format("ExcludeClipboardContentFromMonitorProcessing"), &no)
                && put(format("CanIncludeInClipboardHistory"), &no)
                && put(format("CanUploadToCloudClipboard"), &no)
        };
        unsafe {
            if !ok {
                EmptyClipboard();
            }
            CloseClipboard();
        }
        ok.then(|| unsafe { GetClipboardSequenceNumber() }).ok_or_else(|| "Could not copy to the clipboard.".into())
    }

    pub fn clear_if_unchanged(seq: u32) {
        if unsafe { GetClipboardSequenceNumber() } != seq {
            return; // something else was copied since: leave it alone
        }
        if open() {
            unsafe {
                if GetClipboardSequenceNumber() == seq {
                    EmptyClipboard();
                }
                CloseClipboard();
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn set(_: &str) -> Result<u32, String> {
        Err("Copying secrets is only supported on Windows.".into())
    }
    pub fn clear_if_unchanged(_: u32) {}
}

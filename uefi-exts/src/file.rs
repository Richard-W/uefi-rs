use alloc::{alloc::alloc, boxed::Box};
use core::{alloc::Layout, mem, slice};
use uefi::prelude::*;
use uefi::proto::media::file::{File, FileProtocolInfo};
use uefi::Result;

/// Utility functions for dealing with file data
pub trait FileExt: File {
    /// Get the dynamically allocated info for a file
    fn get_boxed_info<Info: FileProtocolInfo + ?Sized>(&mut self) -> Result<Box<Info>> {
        // Initially try get_info with an empty array, this should always fail
        // as all Info types at least need room for a null-terminator.
        let size = match self
            .get_info::<Info>(&mut [])
            .expect_error("zero sized get_info unexpectedly succeeded")
            .split()
        {
            (s, None) => return Err(s.into()),
            (_, Some(size)) => size,
        };

        // These unsafe alloc APIs make sure our buffer is correctly aligned. We
        // round up a size must always be a multiple of alignment. We turn the
        // pointer into a Box<[u8]>, so it's always freed on error.
        let layout = Layout::from_size_align(size, Info::alignment())
            .unwrap()
            .pad_to_align()
            .unwrap();
        let buffer_start = unsafe { alloc(layout) };
        let mut buffer = unsafe { Box::from_raw(slice::from_raw_parts_mut(buffer_start, size)) };

        let info = self
            .get_info(&mut buffer)
            .discard_errdata()?
            .map(|info_ref| {
                // This operation is safe because info uses the exact memory
                // of the provied buffer (so no memory is leaked), and the box
                // is created if and only if buffer is leaked (so no memory can
                // ever be freed twice).

                assert_eq!(mem::size_of_val(info_ref), layout.size());
                assert_eq!(info_ref as *const Info as *const u8, buffer_start);
                unsafe { Box::from_raw(info_ref as *mut _) }
            });
        mem::forget(buffer);

        Ok(info)
    }
}

impl<T: File> FileExt for T {}

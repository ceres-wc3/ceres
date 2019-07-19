#![feature(custom_attribute)]

use std::ffi::{c_void, CString, CStr};
use std::os::raw;
use std::ptr;
use std::mem::MaybeUninit;
use std::marker::PhantomData;
use std::io::Write;

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use failure::Fail;

use storm_sys as storm;

const ASTERISK: &[u8] = b"*\0";

#[cfg(test)]
mod test;

#[derive(FromPrimitive, Clone, Copy, Debug)]
pub enum GenericErrorCode {
    NoError             = 0,
    FileNotFound        = 2,
    AccessDenied        = 1,
    InvalidHandle       = 9,
    NotEnoughMemory     = 12,
    NotSupported        = 95,
    InvalidParameter    = 22,
    DiskFull            = 28,
    AlreadyExists       = 17,
    InsufficientBuffer  = 105,
    BadFormat           = 1000,
    NoMoreFiles         = 1001,
    HandleEof           = 1002,
    CanNotComplete      = 1003,
    FileCorrupt         = 1004,
    AviFile             = 10000,
    UnknownFileKey      = 10001,
    ChecksumError       = 10002,
    InternalFile        = 10003,
    BaseFileMissing     = 10004,
    MarkedForDelete     = 10005,
    FileIncomplete      = 10006,
    UnknownFileNames    = 10007,
    CantFindPatchPrefix = 10008,
}

#[derive(Debug, Fail)]
pub enum GenericError {
    #[fail(display = "Success")]
    Success,
    #[fail(display = "Error code {:?}", _0)]
    ErrorCode(GenericErrorCode),
    #[fail(display = "Unknown error code {:?}", _0)]
    Unknown(u32),
}

fn get_last_generic_error() -> GenericError {
    let error_code_id = unsafe { storm::GetLastError() };

    let error_code: Option<GenericErrorCode> = FromPrimitive::from_u32(error_code_id);

    if let Some(error_code) = error_code {
        match error_code {
            GenericErrorCode::NoError => GenericError::Success,
            error_code => GenericError::ErrorCode(error_code),
        }
    } else {
        GenericError::Unknown(error_code_id)
    }
}

fn test_for_generic_error() -> Result<(), GenericError> {
    let error = get_last_generic_error();

    if let GenericError::Success = error {
        Ok(())
    } else {
        Err(error)
    }
}

impl std::string::ToString for GenericErrorCode {
    fn to_string(&self) -> String {
        match self {
            GenericErrorCode::NoError => "No error".into(),
            GenericErrorCode::FileNotFound => "File not found".into(),
            GenericErrorCode::AccessDenied => "Access denied".into(),
            _ => format!("Error Code: {}", *self as u32),
        }
    }
}

#[derive(FromPrimitive)]
pub enum SignatureErrorKind {
    NoSignature          = 0,
    VerifyFailed         = 1,
    WeakSignatureOk      = 2,
    WeakSignatureError   = 3,
    StrongSignatureOk    = 4,
    StrongSignatureError = 5,
}

pub struct MPQArchive {
    handle: storm::HANDLE,
}

pub struct MPQFile<'mpq> {
    handle: storm::HANDLE,

    _marker: PhantomData<&'mpq MPQArchive>,
}

pub struct MPQFileIterator<'mpq> {
    archive:       &'mpq MPQArchive,
    search_handle: Option<storm::HANDLE>,
    exhausted:     bool,
}

impl MPQArchive {
    pub fn open<P: AsRef<[u8]>>(path: P) -> Result<MPQArchive, GenericError> {
        const PREFIX: &'static [u8] = b"flat-file://";
        let path = path.as_ref();
        let mut path_buf = Vec::with_capacity(path.len() + PREFIX.len());
        path_buf.write(PREFIX).unwrap();
        path_buf.write(path).unwrap();

        let path = CString::new(path_buf).unwrap();
        let path_ptr = path.as_ptr();
        let mut handle: MaybeUninit<storm::HANDLE> = MaybeUninit::uninit();

        unsafe {
            storm::SFileOpenArchive(path_ptr, 0, 0, handle.as_mut_ptr());
        }

        test_for_generic_error()?;

        Ok(MPQArchive {
            handle: unsafe { handle.assume_init() },
        })
    }

    pub fn open_file<'mpq, P: Into<Vec<u8>>>(
        &'mpq self,
        file_name: P,
    ) -> Result<MPQFile<'mpq>, GenericError> {
        let file_name = CString::new(file_name.into()).unwrap();
        let file_name_ptr = file_name.as_ptr();
        let mut handle: MaybeUninit<storm::HANDLE> = MaybeUninit::uninit();

        unsafe {
            storm::SFileOpenFileEx(self.handle, file_name_ptr, 0, handle.as_mut_ptr());
        }

        test_for_generic_error()?;

        Ok(MPQFile {
            _marker: PhantomData,
            handle:  unsafe { handle.assume_init() },
        })
    }

    pub fn write_file<P: Into<Vec<u8>>, D: AsRef<[u8]>>(
        &self,
        file_name: P,
        data: D,
    ) -> Result<(), GenericError> {
        let file_name = CString::new(file_name).unwrap();
        let file_name_ptr = file_name.as_ptr();
        let data = data.as_ref();
        let mut handle: MaybeUninit<storm::HANDLE> = MaybeUninit::uninit();

        unsafe {
            if !storm::SFileCreateFile(
                self.handle,
                file_name_ptr,
                0,
                data.len() as u32,
                0,
                storm::MPQ_FILE_REPLACEEXISTING,
                handle.as_mut_ptr(),
            ) {
                test_for_generic_error()?;
            }
        }

        let handle = unsafe { handle.assume_init() };

        unsafe {
            if !storm::SFileWriteFile(handle, data.as_ptr() as *const c_void, data.len() as u32, 0)
            {
                test_for_generic_error()?;
            }
        }

        unsafe {
            if !storm::SFileFinishFile(handle) {
                test_for_generic_error()?;
            }
        }

        Ok(())
    }

    pub fn iter_files(&self) -> Result<MPQFileIterator, GenericError> {
        Ok(MPQFileIterator {
            archive:       self,
            search_handle: None,
            exhausted:     false,
        })
    }
}

impl Drop for MPQArchive {
    fn drop(&mut self) {
        unsafe {
            storm::SFileCloseArchive(self.handle);
        }
    }
}

impl<'mpq> MPQFile<'mpq> {
    pub fn get_size(&self) -> Result<u32, GenericError> {
        let mut file_size_high = 0;

        let file_size_low = unsafe { storm::SFileGetFileSize(self.handle, &mut file_size_high) };

        test_for_generic_error()?;

        Ok(file_size_low)
    }

    pub fn read_contents(&self) -> Result<Vec<u8>, GenericError> {
        let size = self.get_size()?;
        let mut buffer: Vec<u8> = Vec::new();
        buffer.resize_with(size as usize, || 0);

        let buffer_ptr = buffer.as_mut_ptr() as *mut c_void;
        let mut bytes_read: u32 = 0;

        unsafe {
            if !storm::SFileReadFile(
                self.handle,
                buffer_ptr,
                size,
                &mut bytes_read,
                ptr::null_mut(),
            ) {
                test_for_generic_error()?;
            }
        }

        Ok(buffer)
    }
}

impl<'mpq> Drop for MPQFile<'mpq> {
    fn drop(&mut self) {
        unsafe {
            storm::SFileCloseFile(self.handle);
        }
    }
}

impl<'mpq> MPQFileIterator<'mpq> {
    fn start_search(&mut self) -> Result<storm::SFILE_FIND_DATA, GenericError> {
        let mut file_info: MaybeUninit<storm::SFILE_FIND_DATA> = MaybeUninit::uninit();

        let handle = unsafe {
            storm::SFileFindFirstFile(
                self.archive.handle,
                ASTERISK.as_ptr() as *const raw::c_char,
                file_info.as_mut_ptr(),
                ptr::null(),
            )
        };

        if handle.is_null() {
            test_for_generic_error()?;
        }

        self.search_handle = Some(handle);
        let file_info = unsafe { file_info.assume_init() };

        Ok(file_info)
    }

    fn continue_search(&mut self) -> Result<storm::SFILE_FIND_DATA, GenericError> {
        let mut file_info: MaybeUninit<storm::SFILE_FIND_DATA> = MaybeUninit::uninit();

        let success = unsafe {
            storm::SFileFindNextFile(self.search_handle.unwrap(), file_info.as_mut_ptr())
        };

        if !success {
            test_for_generic_error()?;
        }

        let file_info = unsafe { file_info.assume_init() };

        Ok(file_info)
    }
}

impl<'mpq> Iterator for MPQFileIterator<'mpq> {
    type Item = CString;

    fn next(&mut self) -> Option<Self::Item> {
        if self.exhausted {
            return None;
        }

        let result = if self.search_handle.is_none() {
            self.start_search()
        } else {
            self.continue_search()
        };

        if result.is_err() {
            self.exhausted = true;
            None
        } else {
            let file_info = result.unwrap();
            let file_name = unsafe { CStr::from_ptr(&file_info.cFileName as *const raw::c_char) };
            let file_name = file_name.to_owned();

            Some(file_name)
        }
    }
}

impl<'mpq> Drop for MPQFileIterator<'mpq> {
    fn drop(&mut self) {
        unsafe {
            if let Some(handle) = self.search_handle {
                storm::SFileFindClose(handle);
            }
        }
    }
}

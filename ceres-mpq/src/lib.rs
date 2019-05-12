#![feature(custom_attribute)]

use std::ffi::{c_void, CString};
use std::ptr;
use std::fmt::Display;

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use failure::Fail;

use storm_sys as storm;

#[derive(FromPrimitive, Clone, Copy, Debug)]
pub enum GenericErrorCode {
    NoError = 0,
    FileNotFound = 2,
    AccessDenied = 1,
    InvalidHandle = 9,
    NotEnoughMemory = 12,
    NotSupported = 95,
    InvalidParameter = 22,
    DiskFull = 28,
    AlreadyExists = 17,
    InsufficientBuffer = 105,
    BadFormat = 1000,
    NoMoreFiles = 1001,
    HandleEof = 1002,
    CanNotComplete = 1003,
    FileCorrupt = 1004,
    AviFile = 10000,
    UnknownFileKey = 10001,
    ChecksumError = 10002,
    InternalFile = 10003,
    BaseFileMissing = 10004,
    MarkedForDelete = 10005,
    FileIncomplete = 10006,
    UnknownFileNames = 10007,
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
            GenericErrorCode::NoError => format!("No error"),
            GenericErrorCode::FileNotFound => format!("File not found"),
            GenericErrorCode::AccessDenied => format!("Access denied"),
            _ => format!("Error Code: {}", *self as u32),
        }
    }
}

#[derive(FromPrimitive)]
pub enum SignatureErrorKind {
    NoSignature = 0,
    VerifyFailed = 1,
    WeakSignatureOk = 2,
    WeakSignatureError = 3,
    StrongSignatureOk = 4,
    StrongSignatureError = 5,
}

pub struct MPQArchive {
    handle: storm::HANDLE,
}

pub struct MPQFile {
    handle: storm::HANDLE,
}

impl MPQArchive {
    pub fn open(path: &str) -> Result<MPQArchive, GenericError> {
        let path = CString::new("flat-file://".to_string() + path).unwrap();
        let path_ptr = path.as_ptr();
        let mut handle = ptr::null_mut();

        unsafe {
            storm::SFileOpenArchive(path_ptr, 0, 0, &mut handle);
        }

        test_for_generic_error()?;

        Ok(MPQArchive { handle })
    }

    pub fn open_file(&self, file_name: &str) -> Result<MPQFile, GenericError> {
        let file_name = CString::new(file_name).unwrap();
        let file_name_ptr = file_name.as_ptr();
        let mut handle = ptr::null_mut();

        unsafe {
            storm::SFileOpenFileEx(self.handle, file_name_ptr, 0, &mut handle);
        }

        test_for_generic_error()?;

        Ok(MPQFile { handle })
    }

    pub fn write_file(&self, file_name: &str, data: &[u8]) -> Result<(), GenericError> {
        let file_name = CString::new(file_name).unwrap();
        let file_name_ptr = file_name.as_ptr();
        let mut handle = ptr::null_mut();

        unsafe {
            if !storm::SFileCreateFile(
                self.handle,
                file_name_ptr,
                0,
                data.len() as u32,
                0,
                storm::MPQ_FILE_REPLACEEXISTING,
                &mut handle,
            ) {
                test_for_generic_error()?;
            }
        }

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
}

impl Drop for MPQArchive {
    fn drop(&mut self) {
        unsafe {
            storm::SFileCloseArchive(self.handle);
        }
    }
}

impl MPQFile {
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

impl Drop for MPQFile {
    fn drop(&mut self) {
        unsafe {
            storm::SFileCloseFile(self.handle);
        }
    }
}

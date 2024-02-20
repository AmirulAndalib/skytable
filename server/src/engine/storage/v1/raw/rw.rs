/*
 * Created on Tue Jul 23 2023
 *
 * This file is a part of Skytable
 * Skytable (formerly known as TerrabaseDB or Skybase) is a free and open-source
 * NoSQL database written by Sayan Nandan ("the Author") with the
 * vision to provide flexibility in data modelling without compromising
 * on performance, queryability or scalability.
 *
 * Copyright (c) 2023, Sayan Nandan <ohsayan@outlook.com>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 *
*/

use {
    crate::{
        engine::{
            error::RuntimeResult,
            storage::common::{
                checksum::SCrc64,
                interface::fs_traits::{
                    FSInterface, FileInterface, FileInterfaceExt, FileInterfaceRead,
                    FileInterfaceWrite, FileInterfaceWriteExt,
                },
                sdss,
            },
        },
        util::os::SysIOError,
    },
    std::marker::PhantomData,
};

pub struct TrackedWriter<Fs: FSInterface> {
    file: SDSSFileIO<Fs, <Fs::File as FileInterface>::BufWriter>,
    _cs: SCrc64,
}

impl<Fs: FSInterface> TrackedWriter<Fs> {
    pub fn new(f: SDSSFileIO<Fs>) -> RuntimeResult<Self> {
        Ok(Self {
            file: f.into_buffered_writer()?,
            _cs: SCrc64::new(),
        })
    }
    pub fn sync_into_inner(self) -> RuntimeResult<SDSSFileIO<Fs>> {
        self.file.downgrade_writer()
    }
}

/// [`SDSSFileLenTracked`] simply maintains application level length and checksum tracking to avoid frequent syscalls because we
/// do not expect (even though it's very possible) users to randomly modify file lengths while we're reading them
pub struct TrackedReader<Fs: FSInterface> {
    f: SDSSFileIO<Fs, <Fs::File as FileInterface>::BufReader>,
    len: u64,
    cursor: u64,
    cs: SCrc64,
}

impl<Fs: FSInterface> TrackedReader<Fs> {
    /// Important: this will only look at the data post the current cursor!
    pub fn new(mut f: SDSSFileIO<Fs>) -> RuntimeResult<Self> {
        let len = f.file_length()?;
        let pos = f.file_cursor()?;
        let f = f.into_buffered_reader()?;
        Ok(Self {
            f,
            len,
            cursor: pos,
            cs: SCrc64::new(),
        })
    }
    pub fn remaining(&self) -> u64 {
        self.len - self.cursor
    }
    pub fn is_eof(&self) -> bool {
        self.len == self.cursor
    }
    pub fn has_left(&self, v: u64) -> bool {
        self.remaining() >= v
    }
    pub fn tracked_read(&mut self, buf: &mut [u8]) -> RuntimeResult<()> {
        self.untracked_read(buf).map(|_| self.cs.update(buf))
    }
    pub fn read_byte(&mut self) -> RuntimeResult<u8> {
        let mut buf = [0u8; 1];
        self.tracked_read(&mut buf).map(|_| buf[0])
    }
    pub fn __reset_checksum(&mut self) -> u64 {
        let mut crc = SCrc64::new();
        core::mem::swap(&mut crc, &mut self.cs);
        crc.finish()
    }
    pub fn untracked_read(&mut self, buf: &mut [u8]) -> RuntimeResult<()> {
        if self.remaining() >= buf.len() as u64 {
            match self.f.read_buffer(buf) {
                Ok(()) => {
                    self.cursor += buf.len() as u64;
                    Ok(())
                }
                Err(e) => return Err(e),
            }
        } else {
            Err(SysIOError::from(std::io::ErrorKind::InvalidInput).into())
        }
    }
    pub fn into_inner_file(self) -> RuntimeResult<SDSSFileIO<Fs>> {
        self.f.downgrade_reader()
    }
    pub fn read_block<const N: usize>(&mut self) -> RuntimeResult<[u8; N]> {
        if !self.has_left(N as _) {
            return Err(SysIOError::from(std::io::ErrorKind::InvalidInput).into());
        }
        let mut buf = [0; N];
        self.tracked_read(&mut buf)?;
        Ok(buf)
    }
    pub fn read_u64_le(&mut self) -> RuntimeResult<u64> {
        Ok(u64::from_le_bytes(self.read_block()?))
    }
}

#[derive(Debug)]
pub struct SDSSFileIO<Fs: FSInterface, F = <Fs as FSInterface>::File> {
    f: F,
    _fs: PhantomData<Fs>,
}

impl<Fs: FSInterface> SDSSFileIO<Fs> {
    pub fn open<F: sdss::sdss_r1::FileSpecV1<DecodeArgs = ()>>(
        fpath: &str,
    ) -> RuntimeResult<(Self, F::Metadata)> {
        let mut f = Self::_new(Fs::fs_fopen_rw(fpath)?);
        let v = F::read_metadata(&mut f.f, ())?;
        Ok((f, v))
    }
    pub fn into_buffered_reader(
        self,
    ) -> RuntimeResult<SDSSFileIO<Fs, <Fs::File as FileInterface>::BufReader>> {
        self.f.upgrade_to_buffered_reader().map(SDSSFileIO::_new)
    }
    pub fn into_buffered_writer(
        self,
    ) -> RuntimeResult<SDSSFileIO<Fs, <Fs::File as FileInterface>::BufWriter>> {
        self.f.upgrade_to_buffered_writer().map(SDSSFileIO::_new)
    }
}

impl<Fs: FSInterface> SDSSFileIO<Fs, <Fs::File as FileInterface>::BufReader> {
    pub fn downgrade_reader(self) -> RuntimeResult<SDSSFileIO<Fs, Fs::File>> {
        let me = <Fs::File as FileInterface>::downgrade_reader(self.f)?;
        Ok(SDSSFileIO::_new(me))
    }
}

impl<Fs: FSInterface> SDSSFileIO<Fs, <Fs::File as FileInterface>::BufWriter> {
    pub fn downgrade_writer(self) -> RuntimeResult<SDSSFileIO<Fs>> {
        let me = <Fs::File as FileInterface>::downgrade_writer(self.f)?;
        Ok(SDSSFileIO::_new(me))
    }
}

impl<Fs: FSInterface, F> SDSSFileIO<Fs, F> {
    fn _new(f: F) -> Self {
        Self {
            f,
            _fs: PhantomData,
        }
    }
}

impl<Fs: FSInterface, F: FileInterfaceRead> SDSSFileIO<Fs, F> {
    pub fn read_buffer(&mut self, buffer: &mut [u8]) -> RuntimeResult<()> {
        self.f.fread_exact(buffer)
    }
}

impl<Fs: FSInterface, F: FileInterfaceExt> SDSSFileIO<Fs, F> {
    pub fn file_cursor(&mut self) -> RuntimeResult<u64> {
        self.f.fext_cursor()
    }
    pub fn file_length(&self) -> RuntimeResult<u64> {
        self.f.fext_length()
    }
    pub fn seek_from_start(&mut self, by: u64) -> RuntimeResult<()> {
        self.f.fext_seek_ahead_from_start_by(by)
    }
}

impl<Fs: FSInterface, F: FileInterfaceRead + FileInterfaceExt> SDSSFileIO<Fs, F> {
    pub fn read_full(&mut self) -> RuntimeResult<Vec<u8>> {
        let len = self.file_length()? - self.file_cursor()?;
        let mut buf = vec![0; len as usize];
        self.read_buffer(&mut buf)?;
        Ok(buf)
    }
}

impl<Fs: FSInterface, F: FileInterfaceWrite + FileInterfaceWriteExt> SDSSFileIO<Fs, F> {
    pub fn fsynced_write(&mut self, data: &[u8]) -> RuntimeResult<()> {
        self.f.fw_write_all(data)?;
        self.f.fwext_sync_all()
    }
}

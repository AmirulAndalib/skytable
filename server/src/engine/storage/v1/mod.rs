/*
 * Created on Mon May 15 2023
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

// impls
mod batch_jrnl;
mod journal;
pub(in crate::engine) mod loader;
mod rw;
pub mod spec;
mod sysdb;
// hl
pub mod inf;
// test
pub mod memfs;
#[cfg(test)]
mod tests;

// re-exports
pub use {
    journal::{open_journal, JournalAdapter, JournalWriter},
    memfs::NullFS,
    rw::{LocalFS, RawFSInterface, SDSSFileIO},
};
pub mod data_batch {
    pub use super::batch_jrnl::{create, reinit, DataBatchPersistDriver, DataBatchRestoreDriver};
}

use crate::{
    engine::{
        error::{CtxError, CtxResult},
        txn::TransactionError,
    },
    util::os::SysIOError as IoError,
};

pub type SDSSResult<T> = CtxResult<T, SDSSErrorKind>;
pub type SDSSError = CtxError<SDSSErrorKind>;

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub enum SDSSErrorKind {
    // IO errors
    /// An IO err
    IoError(IoError),
    OtherError(&'static str),
    CorruptedFile(&'static str),
    // header
    /// version mismatch
    HeaderDecodeVersionMismatch,
    /// The entire header is corrupted
    HeaderDecodeCorruptedHeader,
    /// Expected header values were not matched with the current header
    HeaderDecodeDataMismatch,
    /// The time in the [header/dynrec/rtsig] is in the future
    HeaderTimeConflict,
    // journal
    /// While attempting to handle a basic failure (such as adding a journal entry), the recovery engine ran into an exceptional
    /// situation where it failed to make a necessary repair the log
    JournalWRecoveryStageOneFailCritical,
    /// An entry in the journal is corrupted
    JournalLogEntryCorrupted,
    /// The structure of the journal is corrupted
    JournalCorrupted,
    // internal file structures
    /// While attempting to decode a structure in an internal segment of a file, the storage engine ran into a possibly irrecoverable error
    InternalDecodeStructureCorrupted,
    /// the payload (non-static) part of a structure in an internal segment of a file is corrupted
    InternalDecodeStructureCorruptedPayload,
    /// the data for an internal structure was decoded but is logically invalid
    InternalDecodeStructureIllegalData,
    /// when attempting to flush a data batch, the batch journal crashed and a recovery event was triggered. But even then,
    /// the data batch journal could not be fixed
    DataBatchRecoveryFailStageOne,
    /// when attempting to restore a data batch from disk, the batch journal crashed and had a corruption, but it is irrecoverable
    DataBatchRestoreCorruptedBatch,
    /// when attempting to restore a data batch from disk, the driver encountered a corrupted entry
    DataBatchRestoreCorruptedEntry,
    /// we failed to close the data batch
    DataBatchCloseError,
    DataBatchRestoreCorruptedBatchFile,
    JournalRestoreTxnError,
    SysDBCorrupted,
}

impl From<TransactionError> for SDSSErrorKind {
    fn from(_: TransactionError) -> Self {
        Self::JournalRestoreTxnError
    }
}

direct_from! {
    SDSSErrorKind => {
        std::io::Error as IoError,
        IoError as IoError,
    }
}

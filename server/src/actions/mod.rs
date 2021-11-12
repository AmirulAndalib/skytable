/*
 * Created on Wed Aug 19 2020
 *
 * This file is a part of Skytable
 * Skytable (formerly known as TerrabaseDB or Skybase) is a free and open-source
 * NoSQL database written by Sayan Nandan ("the Author") with the
 * vision to provide flexibility in data modelling without compromising
 * on performance, queryability or scalability.
 *
 * Copyright (c) 2020, Sayan Nandan <ohsayan@outlook.com>
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

//! # Actions
//!
//! Actions are like shell commands, you provide arguments -- they return output. This module contains a collection
//! of the actions supported by Skytable
//!

#[macro_use]
mod macros;
pub mod dbsize;
pub mod del;
pub mod exists;
pub mod flushdb;
pub mod get;
pub mod jget;
pub mod keylen;
pub mod lists;
pub mod lskeys;
pub mod mget;
pub mod mpop;
pub mod mset;
pub mod mupdate;
pub mod pop;
pub mod set;
pub mod strong;
pub mod update;
pub mod uset;
pub mod whereami;
pub mod heya {
    //! Respond to `HEYA` queries
    use crate::dbnet::connection::prelude::*;
    use crate::resp::BytesWrapper;
    action!(
        /// Returns a `HEY!` `Response`
        fn heya(_handle: &Corestore, con: &'a mut T, mut act: ActionIter<'a>) {
            err_if_len_is!(act, con, gt 1);
            if act.len() == 1 {
                let raw_byte = unsafe { act.next_unchecked_bytes() };
                con.write_response(BytesWrapper(raw_byte)).await
            } else {
                con.write_response(responses::groups::HEYA).await
            }
        }
    );
}

// Copyright (C) 2025 Ethan Uppal.
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU General Public License as published by the Free Software
// Foundation, version 3 of the License only.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU General Public License for more
// details.
//
// You should have received a copy of the GNU General Public License along with
// this program.  If not, see <https://www.gnu.org/licenses/>.

use std::{env, io};

use wine::{Prefix, PrefixConfig};

pub fn main() -> io::Result<()> {
    let prefix_path = env::args().nth(1).expect("usage: <path to wine prefix>");
    let prefix =
        Prefix::at(&prefix_path, ["/usr/local/lib"], PrefixConfig::default());
    prefix.kill_all()?;
    Ok(())
}

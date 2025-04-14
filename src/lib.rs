// Copyright (C) 2025 Ethan Uppal.
//
// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    ffi::{OsStr, OsString},
    io,
    path::{Path, PathBuf},
    process::{self, Command},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugClass {
    Trace,
    Warn,
    Error,
    Fixme,
}

impl DebugClass {
    pub fn as_os_str(&self) -> &OsStr {
        OsStr::new(match self {
            Self::Trace => "trace",
            Self::Warn => "warn",
            Self::Error => "err",
            Self::Fixme => "fixme",
        })
    }
}

// $ rg -g '*.c' -g '*.h' '^.*WINE_(DEFAULT|DECLARE)_DEBUG_CHANNEL\(([^)]+)\).*'
// -or '$2' --no-filename dlls/ programs/ |  awk '!seen[$0]++'
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugChannel<'a> {
    All,
    Heap,
    LoadDll,
    Module,
    Pid,
    Relay,
    Seh,
    Server,
    Snoop,
    Synchronous,
    Timestamp,
    Fps,
    DebugString,
    ThreadName,
    Other(&'a str),
}

impl DebugChannel<'_> {
    pub fn as_os_str(&self) -> &OsStr {
        OsStr::new(match self {
            Self::All => "all",
            Self::Heap => "heap",
            Self::LoadDll => "loaddll",
            Self::Module => "module",
            Self::Pid => "pid",
            Self::Relay => "relay",
            Self::Seh => "seh",
            Self::Server => "server",
            Self::Snoop => "snoop",
            Self::Synchronous => "synchronous",
            Self::Timestamp => "timestamp",
            Self::Fps => "fps",
            Self::DebugString => "debugstr",
            Self::ThreadName => "threadname",
            Self::Other(other) => other,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugRule<'a> {
    pub process: Option<&'a OsStr>,
    pub class: Option<DebugClass>,
    pub channel: DebugChannel<'a>,
    pub is_enabled: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct DebugRules<'a> {
    pub rules: Vec<DebugRule<'a>>,
}

impl<'a> DebugRules<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&'a mut self, rule: DebugRule<'a>) -> &'a mut Self {
        self.rules.push(rule);
        self
    }

    pub fn enable(&'a mut self, channel: DebugChannel<'a>) -> &'a mut Self {
        self.rules.push(DebugRule {
            process: None,
            class: None,
            channel,
            is_enabled: true,
        });
        self
    }

    pub fn disable(&'a mut self, channel: DebugChannel<'a>) -> &'a mut Self {
        self.rules.push(DebugRule {
            process: None,
            class: None,
            channel,
            is_enabled: false,
        });
        self
    }
}

impl<'a> AsRef<DebugRules<'a>> for DebugRules<'a> {
    fn as_ref(&self) -> &DebugRules<'a> {
        self
    }
}

#[derive(Debug, Default, PartialEq, Eq, Hash, Clone)]
pub struct PrefixConfig {
    pub esync: bool,
    pub msync: bool,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Prefix {
    path: PathBuf,
    dynamic_library_paths: OsString,

    wine: OsString,
    wineserver: OsString,
    regedit: OsString,

    config: PrefixConfig,
}

impl Prefix {
    pub fn at(
        path: impl AsRef<Path>,
        dynamic_library_paths: impl IntoIterator<Item = impl AsRef<Path>>,
        config: PrefixConfig,
    ) -> Self {
        let path = path.as_ref().to_path_buf();
        let wine: OsString = path.join("bin/wine").into();
        let wineserver = path.join("bin/wineserver").into();
        let regedit = path.join("bin/regedit").into();

        assert!(
            Path::new(&wine).is_file(),
            "Invalid prefix (todo better error handling)"
        );

        Self {
            path,
            dynamic_library_paths: dynamic_library_paths
                .into_iter()
                .enumerate()
                .fold(OsString::new(), |mut acc, (i, into_cow)| {
                    if i > 0 {
                        acc.push(":");
                    }
                    acc.push(into_cow.as_ref());
                    acc
                }),
            wine,
            wineserver,
            regedit,
            config,
        }
    }

    pub fn command<'b>(
        &'b self,
        use_start_exe: bool,
        program: impl AsRef<OsStr>,
        debug_rules: impl AsRef<DebugRules<'b>>,
    ) -> Command {
        let debug_rules = debug_rules.as_ref();
        let mut command = Command::new(&self.wine);

        command.current_dir(&self.path);

        command.envs([
            ("WINEPREFIX", self.path.as_os_str()),
            ("DYLD_FALLBACK_LIBRARY_PATH", &self.dynamic_library_paths),
        ]);
        if self.config.esync {
            command.env("ESYNC", "1");
        }
        if self.config.msync {
            command.env("MSYNC", "1");
        }
        if !debug_rules.rules.is_empty() {
            let mut debug_env_value = OsString::new();
            for (i, debug_rule) in debug_rules.rules.iter().enumerate() {
                if i > 0 {
                    debug_env_value.push(",");
                }
                if let Some(process) = &debug_rule.process {
                    debug_env_value.push(process);
                    debug_env_value.push(":");
                }
                if let Some(class) = &debug_rule.class {
                    debug_env_value.push(class.as_os_str());
                    debug_env_value.push(":");
                }
                debug_env_value.push(if debug_rule.is_enabled {
                    "+"
                } else {
                    "-"
                });
                debug_env_value.push(debug_rule.channel.as_os_str());
            }
            command.env("WINEDEBUG", debug_env_value);
        }

        if use_start_exe {
            command.arg("start");
        }
        command.arg(program);

        command
    }

    pub fn kill_all(&self) -> io::Result<process::Output> {
        Command::new(&self.wineserver)
            .current_dir(&self.path)
            .env("WINEPREFIX", self.path.as_os_str())
            .arg("-k")
            .output()
    }
}

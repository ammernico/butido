//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::io::IsTerminal;

use serde::Deserialize;
use serde::Serialize;

#[derive(
    parse_display::Display,
    Serialize,
    Deserialize,
    Clone,
    Debug,
    Hash,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
)]
#[serde(transparent)]
#[display("{0}")]
pub struct EnvironmentVariableName(String);

impl From<&str> for EnvironmentVariableName {
    fn from(s: &str) -> EnvironmentVariableName {
        EnvironmentVariableName(s.to_string())
    }
}

impl AsRef<str> for EnvironmentVariableName {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

pub mod docker;
pub mod env;
pub mod filters;
pub mod git;
pub mod parser;
pub mod progress;

pub fn stdout_is_pipe() -> bool {
    !std::io::stdout().is_terminal()
}

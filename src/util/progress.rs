//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use getset::CopyGetters;
use indicatif::*;

#[derive(Clone, Debug, CopyGetters)]
pub struct ProgressBars {
    _bar_template: String,

    #[getset(get_copy = "pub")]
    hide: bool,
}

impl ProgressBars {
    pub fn setup(_bar_template: String, hide: bool) -> Self {
        ProgressBars {
            _bar_template,
            hide,
        }
    }

    pub fn bar(&self) -> anyhow::Result<ProgressBar> {
        Ok(ProgressBar::hidden())
    }
}

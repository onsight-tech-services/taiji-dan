//  Copyright 2022 OnSight Tech Services LLC
//  SPDX-License-Identifier: BSD-3-Clause

use taiji_engine_types::TemplateAddress;

pub trait TemplateProvider: Send + Sync + Clone + 'static {
    type Template;
    type Error: std::error::Error + Sync + Send + 'static;

    fn get_template_module(&self, id: &TemplateAddress) -> Result<Option<Self::Template>, Self::Error>;
}

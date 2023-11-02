// Copyright 2022 OnSight Tech Services LLC
// SPDX-License-Identifier: BSD-3-Clause

use std::collections::HashMap;

use d3ne::{Node, OutputValue, Worker};
use taiji_dan_common_types::services::template_provider::TemplateProvider;

use crate::{flow::FlowContext, function_definitions::ArgType, packager::LoadedTemplate};

pub struct ArgWorker {}

impl<TTemplateProvider: TemplateProvider<Template = LoadedTemplate>> Worker<FlowContext<TTemplateProvider>>
    for ArgWorker
{
    fn work(
        &self,
        context: &FlowContext<TTemplateProvider>,
        node: &Node,
        _input_data: HashMap<String, OutputValue>,
    ) -> Result<HashMap<String, OutputValue>, anyhow::Error> {
        let arg_name: String = node
            .get_data("name")?
            .ok_or_else(|| anyhow::anyhow!("could not find arg `name`"))?;
        let (value, arg_def) = context
            .args
            .get(arg_name.as_str())
            .ok_or_else(|| anyhow::anyhow!("could not find arg"))?;

        let mut result = HashMap::new();
        match arg_def.arg_type {
            ArgType::String => {
                result.insert(
                    "default".to_string(),
                    OutputValue::String(String::from_utf8(value.clone())?),
                );
            },
            ArgType::Bytes => {
                result.insert("default".to_string(), OutputValue::Bytes(value.clone()));
            },
        };
        Ok(result)
    }

    fn name(&self) -> &str {
        "taiji::arg"
    }
}

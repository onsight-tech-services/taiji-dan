// Copyright 2022 OnSight Tech Services LLC
// SPDX-License-Identifier: BSD-3-Clause

use std::{collections::HashMap, sync::Arc};

use d3ne::{Engine, Node, Workers, WorkersBuilder};
use serde_json::Value as JsValue;
use taiji_dan_common_types::services::template_provider::TemplateProvider;
use taiji_engine_types::instruction_result::InstructionResult;
use taiji_template_lib::args::Arg;

use crate::{
    flow::{
        workers::{ArgWorker, CallMethodWorker},
        FlowContext,
        FlowEngineError,
    },
    function_definitions::FunctionArgDefinition,
    packager::LoadedTemplate,
    runtime::{AuthorizationScope, Runtime},
};

#[derive(Clone, Debug)]
pub struct FlowInstance {
    // engine: Engine,
    // TODO: engine is not Send so can't be added here
    // process: JsValue,
    start_node: i64,
    nodes: HashMap<i64, Node>,
}

impl FlowInstance {
    pub fn try_build<TTemplateProvider: TemplateProvider<Template = LoadedTemplate>>(
        value: JsValue,
        workers: Workers<FlowContext<TTemplateProvider>>,
    ) -> Result<Self, FlowEngineError> {
        let engine = Engine::new("taiji_engine@0.1.0".to_string(), workers);
        // dbg!(&value);
        let nodes = engine.parse_value(value).expect("could not create engine");
        Ok(FlowInstance {
            // process: value,
            nodes,
            start_node: 1,
        })
    }

    pub fn invoke<TTemplateProvider: TemplateProvider<Template = LoadedTemplate>>(
        &self,
        template_provider: Arc<TTemplateProvider>,
        runtime: Runtime,
        auth_scope: AuthorizationScope,
        args: &[Arg],
        arg_defs: &[FunctionArgDefinition],
        recursion_depth: usize,
        max_recursion_depth: usize,
    ) -> Result<InstructionResult, FlowEngineError> {
        let engine = Engine::new("taiji@0.1.0".to_string(), load_workers());
        let args = runtime.resolve_args(args.to_vec())?;
        let mut args_map = HashMap::new();
        for (i, arg_def) in arg_defs.iter().enumerate() {
            if i >= args.len() {
                return Err(FlowEngineError::MissingArgument {
                    name: arg_def.name.clone(),
                });
            }
            args_map.insert(arg_def.name.clone(), (args[i].clone(), arg_def.clone()));
        }

        let context = FlowContext {
            template_provider,
            runtime,
            auth_scope,
            args: args_map,
            recursion_depth,
            max_recursion_depth,
        };
        let result = engine.process(&context, &self.nodes, self.start_node)?;
        if result.is_empty() {
            Ok(InstructionResult::empty())
        } else {
            todo!("Returning results from a flow is not yet implemented")
        }
    }
}

fn load_workers<TTemplateProvider: TemplateProvider<Template = LoadedTemplate>>(
) -> Workers<FlowContext<TTemplateProvider>> {
    let mut workers = WorkersBuilder::default();

    // workers.add(StartWorker {});
    workers.add(CallMethodWorker {});
    // workers.add(CreateBucketWorker {
    //     state_db: state_db.clone(),
    // });
    // workers.add(StoreBucketWorker {
    //     state_db: state_db.clone(),
    // });
    // workers.add(ArgWorker { args: args.clone() });
    workers.add(ArgWorker {});
    // workers.add(SenderWorker { sender });
    // workers.add(TextWorker {});
    // workers.add(HasRoleWorker { state_db });
    // workers.add(MintBucketWorker {});
    workers.build()
}

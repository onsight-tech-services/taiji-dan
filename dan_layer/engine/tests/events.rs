//   Copyright 2023 OnSight Tech Services LLC
//   SPDX-License-Identifier: BSD-3-Clause

use taiji_engine_types::instruction::Instruction;
use taiji_template_lib::args;
use taiji_template_test_tooling::TemplateTest;

#[test]
fn basic_emit_event() {
    let mut template_test = TemplateTest::new(vec!["tests/templates/events"]);
    let event_emitter_template = template_test.get_template_address("EventEmitter");
    let result = template_test
        .execute_and_commit(
            vec![Instruction::CallFunction {
                template_address: event_emitter_template,
                function: "test_function".to_string(),
                args: args![],
            }],
            vec![],
        )
        .expect("Failed to emit test event");
    assert!(result.finalize.is_accept());
    assert_eq!(result.finalize.events.len(), 1);
    assert_eq!(result.finalize.events[0].topic(), "Hello world !");
    assert_eq!(result.finalize.events[0].template_address(), event_emitter_template);
    assert_eq!(result.finalize.events[0].component_address(), None);
    assert_eq!(
        result.finalize.events[0].get_payload("my").unwrap(),
        "event".to_string()
    );
}

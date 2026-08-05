pub mod command;
pub mod keymap;
pub mod predicate;
pub mod registry;

pub use command::Command;
pub use keymap::{Keybinding, Keymap, KeymapDiagnostic, KeymapError};
pub use predicate::{CommandContext, ContextPredicate, PredicateParseError};
pub use registry::{CommandError, CommandHandler, CommandRegistry};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_context_predicate_parser_corpus() {
        let corpus = vec![
            ("panel_focused", ContextPredicate::Flag("panel_focused".into())),
            (
                "selection_non_empty",
                ContextPredicate::Flag("selection_non_empty".into()),
            ),
            ("is_archive", ContextPredicate::Flag("is_archive".into())),
            ("is_remote", ContextPredicate::Flag("is_remote".into())),
            (
                "mode == 'full'",
                ContextPredicate::Eq("mode".into(), "full".into()),
            ),
            (
                "mode == \"full\"",
                ContextPredicate::Eq("mode".into(), "full".into()),
            ),
            (
                "!panel_focused",
                ContextPredicate::Not(Box::new(ContextPredicate::Flag("panel_focused".into()))),
            ),
            (
                "panel_focused && selection_non_empty",
                ContextPredicate::And(vec![
                    ContextPredicate::Flag("panel_focused".into()),
                    ContextPredicate::Flag("selection_non_empty".into()),
                ]),
            ),
            (
                "is_archive || is_remote",
                ContextPredicate::Or(vec![
                    ContextPredicate::Flag("is_archive".into()),
                    ContextPredicate::Flag("is_remote".into()),
                ]),
            ),
            (
                "(panel_focused && selection_non_empty) || mode == 'full'",
                ContextPredicate::Or(vec![
                    ContextPredicate::And(vec![
                        ContextPredicate::Flag("panel_focused".into()),
                        ContextPredicate::Flag("selection_non_empty".into()),
                    ]),
                    ContextPredicate::Eq("mode".into(), "full".into()),
                ]),
            ),
            ("true", ContextPredicate::True),
            ("false", ContextPredicate::False),
        ];

        for (expr, expected) in corpus {
            let parsed = ContextPredicate::parse(expr)
                .unwrap_or_else(|e| panic!("Failed to parse expr '{expr}': {e:?}"));
            assert_eq!(parsed, expected, "Mismatch for expr '{expr}'");
        }
    }

    #[test]
    fn test_context_predicate_evaluator() {
        let pred = ContextPredicate::parse("(panel_focused && selection_non_empty) || mode == 'full'")
            .unwrap();

        let ctx1 = CommandContext::new()
            .with_flag("panel_focused")
            .with_flag("selection_non_empty");
        assert!(ctx1.eval(&pred));

        let ctx2 = CommandContext::new().with_var("mode", "full");
        assert!(ctx2.eval(&pred));

        let ctx3 = CommandContext::new().with_flag("panel_focused");
        assert!(!ctx3.eval(&pred));
    }

    #[test]
    fn test_command_registry_registration_and_dispatch() {
        let mut registry = CommandRegistry::new();

        let pred = ContextPredicate::parse("panel_focused").unwrap();
        let cmd = Command::new("file.copy", "Copy File", "Operations")
            .with_precondition(pred)
            .with_args_schema(json!({ "type": "object" }));

        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        let handler: CommandHandler = Arc::new(move |_ctx, _args| {
            executed_clone.store(true, Ordering::SeqCst);
            Ok(())
        });

        registry.register(cmd, handler).unwrap();

        assert!(registry.get("file.copy").is_some());
        assert_eq!(registry.list().len(), 1);

        // Precondition fails if panel_focused flag is absent
        let ctx_unfocused = CommandContext::new();
        let err = registry.dispatch("file.copy", &ctx_unfocused, json!({}));
        assert!(matches!(err, Err(CommandError::PreconditionFailed(_))));
        assert!(!executed.load(Ordering::SeqCst));

        // Precondition succeeds if panel_focused is present
        let ctx_focused = CommandContext::new().with_flag("panel_focused");
        registry
            .dispatch("file.copy", &ctx_focused, json!({}))
            .unwrap();
        assert!(executed.load(Ordering::SeqCst));
    }

    #[test]
    fn test_keymap_toml_and_tc_parsing_with_conflict_diagnostics() {
        let toml_data = r#"
[[keybinding]]
key = "F5"
command = "file.copy"
context = "panel_focused"

[[keybinding]]
key = "F5"
command = "file.duplicate"
context = "panel_focused"
"#;

        let (keymap, diags) = Keymap::from_toml(toml_data).unwrap();
        assert_eq!(keymap.bindings().len(), 2);
        assert_eq!(diags.len(), 1);
        assert!(matches!(&diags[0], KeymapDiagnostic::Conflict { .. }));

        let tc_data = r#"
; TC keymap comment
F5=cm_CopyFiles
F6=cm_MoveOnly
C+C=cm_CopyToClipboard
CS+F5=cm_PackFiles
"#;

        let (tc_keymap, tc_diags) = Keymap::from_tc_format(tc_data).unwrap();
        assert_eq!(tc_keymap.bindings().len(), 4);
        assert!(tc_diags.is_empty());

        let ctx = CommandContext::new();
        let resolved = tc_keymap.resolve("Ctrl+C", &ctx);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().command_id, "cm_CopyToClipboard");

        let resolved_shift = tc_keymap.resolve("Ctrl+Shift+F5", &ctx);
        assert!(resolved_shift.is_some());
        assert_eq!(resolved_shift.unwrap().command_id, "cm_PackFiles");
    }
}

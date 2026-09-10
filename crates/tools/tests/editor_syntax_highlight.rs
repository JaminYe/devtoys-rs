#![cfg(feature = "gui")]

use devtoys_tools::ui::{highlight_layout_job, labeled_code, labeled_code_editor};

#[test]
fn test_json_highlighting_multiple_spans_and_exact_text() {
    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        let snippet =
            r#"{"name": "DevToys", "version": 1, "enabled": true, "tags": ["tool", "gui"]}"#;
        let job = highlight_layout_job(ui, snippet, Some("json"), 600.0);
        assert_eq!(job.text, snippet);
        assert!(job.sections.len() > 1);
        let colors: std::collections::HashSet<_> =
            job.sections.iter().map(|s| s.format.color).collect();
        assert!(
            colors.len() > 1,
            "JSON snippet should have multiple distinct highlight colors"
        );
    });
    out.textures_delta.clear();
}

#[test]
fn test_xml_highlighting_multiple_spans_and_exact_text() {
    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        let snippet = r#"<manifest package="com.devtoys"><application icon="@res/logo">entry</application></manifest>"#;
        let job = highlight_layout_job(ui, snippet, Some("xml"), 600.0);
        assert_eq!(job.text, snippet);
        assert!(job.sections.len() > 1);
        let colors: std::collections::HashSet<_> =
            job.sections.iter().map(|s| s.format.color).collect();
        assert!(
            colors.len() > 1,
            "XML snippet should have multiple distinct highlight colors"
        );
    });
    out.textures_delta.clear();
}

#[test]
fn test_sql_highlighting_multiple_spans_and_exact_text() {
    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        let snippet = "SELECT u.id, u.username, count(o.id) AS total_orders FROM users u JOIN orders o ON u.id = o.user_id WHERE u.active = 1 GROUP BY u.id;";
        let job = highlight_layout_job(ui, snippet, Some("sql"), 600.0);
        assert_eq!(job.text, snippet);
        assert!(job.sections.len() > 1);
        let colors: std::collections::HashSet<_> =
            job.sections.iter().map(|s| s.format.color).collect();
        assert!(
            colors.len() > 1,
            "SQL snippet should have multiple distinct highlight colors"
        );
    });
    out.textures_delta.clear();
}

#[test]
fn test_malformed_and_incomplete_inputs_do_not_panic() {
    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        let test_cases = [
            ("json", r#"{"unclosed": "string, 123, [true, false"#),
            ("json", r#"}{"broken": : : 999"#),
            ("xml", r#"<unclosed tag attribute="value" <another <<>>"#),
            ("xml", r#"<?xml version="1.0" <unclosed"#),
            ("sql", "SELEC FROM WHERE 1 = 'unclosed string"),
            ("sql", ";;;;;;SELECT ((( FROM WHERE"),
        ];

        for (lang, malformed) in test_cases {
            let job = highlight_layout_job(ui, malformed, Some(lang), 500.0);
            assert_eq!(
                job.text, malformed,
                "Malformed input text must be preserved identically"
            );
        }
    });
    out.textures_delta.clear();
}

#[test]
fn test_dark_and_light_themes_produce_readable_colors() {
    let ctx = egui::Context::default();

    for dark_mode in [true, false] {
        let visuals = if dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        ctx.set_visuals(visuals);

        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let json = r#"{"status": "ok", "code": 200}"#;
            let job = highlight_layout_job(ui, json, Some("json"), 400.0);
            assert_eq!(job.text, json);
            let colors: std::collections::HashSet<_> =
                job.sections.iter().map(|s| s.format.color).collect();
            assert!(
                colors.len() > 1,
                "dark={dark_mode} should produce colored spans"
            );

            let sql = "SELECT * FROM items WHERE price > 10.5;";
            let job_sql = highlight_layout_job(ui, sql, Some("sql"), 400.0);
            assert_eq!(job_sql.text, sql);
            let colors_sql: std::collections::HashSet<_> =
                job_sql.sections.iter().map(|s| s.format.color).collect();
            assert!(
                colors_sql.len() > 1,
                "dark={dark_mode} should produce colored SQL spans"
            );
        });
        out.textures_delta.clear();
    }
}

#[test]
fn test_plaintext_tools_remain_unhighlighted() {
    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        let plain_input = "Line 1\nLine 2 with some words\nLine 3";
        let job = highlight_layout_job(ui, plain_input, None, 500.0);
        assert_eq!(job.text, plain_input);
        assert_eq!(
            job.sections.len(),
            1,
            "Language: None should produce a single unhighlighted section"
        );

        let mut text = plain_input.to_string();
        let changed = labeled_code(ui, "Plain Text", "plain-text-id", &mut text, "hint", true);
        assert!(!changed);
        assert_eq!(text, plain_input);
    });
    out.textures_delta.clear();
}

#[test]
fn test_formatter_views_render_with_syntax_highlighting() {
    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        let mut json_text = r#"{"a": 1}"#.to_string();
        let _ = labeled_code_editor(
            ui,
            "JSON In",
            "test-json-in",
            &mut json_text,
            "hint",
            true,
            Some("json"),
        );

        let mut xml_text = r#"<a x="1">v</a>"#.to_string();
        let _ = labeled_code_editor(
            ui,
            "XML In",
            "test-xml-in",
            &mut xml_text,
            "hint",
            true,
            Some("xml"),
        );

        let mut sql_text = "SELECT 1;".to_string();
        let _ = labeled_code_editor(
            ui,
            "SQL In",
            "test-sql-in",
            &mut sql_text,
            "hint",
            true,
            Some("sql"),
        );
    });
    out.textures_delta.clear();
}

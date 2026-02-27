pub mod speech;
pub mod types;

use types::ToolPreview;

pub fn classify_tool_preview(message: &str) -> ToolPreview {
    let normalized = message.to_lowercase();

    if normalized.contains("open") {
        ToolPreview {
            name: "open_app_or_file".to_string(),
            risk_tier: 1,
            requires_confirmation: false,
            note: "Low-risk open action. Execution engine stub only in Sprint 1.".to_string(),
        }
    } else if normalized.contains("delete") || normalized.contains("remove") {
        ToolPreview {
            name: "delete_file".to_string(),
            risk_tier: 2,
            requires_confirmation: true,
            note: "High-risk action. Confirmation required (policy engine in later sprint)."
                .to_string(),
        }
    } else {
        ToolPreview {
            name: "search_files_semantic".to_string(),
            risk_tier: 0,
            requires_confirmation: false,
            note: "Read-only search action. Stub routing for now.".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::classify_tool_preview;

    #[test]
    fn classifies_open_as_tier_1() {
        let preview = classify_tool_preview("Open downloads");
        assert_eq!(preview.name, "open_app_or_file");
        assert_eq!(preview.risk_tier, 1);
        assert!(!preview.requires_confirmation);
    }

    #[test]
    fn classifies_delete_as_tier_2() {
        let preview = classify_tool_preview("Delete old files");
        assert_eq!(preview.name, "delete_file");
        assert_eq!(preview.risk_tier, 2);
        assert!(preview.requires_confirmation);
    }

    #[test]
    fn classifies_remove_as_tier_2() {
        let preview = classify_tool_preview("remove temp");
        assert_eq!(preview.name, "delete_file");
        assert_eq!(preview.risk_tier, 2);
        assert!(preview.requires_confirmation);
    }

    #[test]
    fn classifies_default_as_read_only_search() {
        let preview = classify_tool_preview("Find project notes");
        assert_eq!(preview.name, "search_files_semantic");
        assert_eq!(preview.risk_tier, 0);
        assert!(!preview.requires_confirmation);
    }

    #[test]
    fn classification_is_case_insensitive() {
        let preview = classify_tool_preview("OPEN THE DOCUMENTS");
        assert_eq!(preview.name, "open_app_or_file");
    }

    #[test]
    fn open_takes_precedence_when_multiple_keywords_exist() {
        let preview = classify_tool_preview("open and delete this file");
        assert_eq!(preview.name, "open_app_or_file");
        assert_eq!(preview.risk_tier, 1);
    }
}

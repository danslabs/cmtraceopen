use crate::collector::types::CollectionProfile;

const EMBEDDED_PROFILE_JSON: &str = include_str!("profile_data.json");

impl CollectionProfile {
    /// Load the embedded collection profile compiled into the binary.
    pub fn embedded() -> CollectionProfile {
        serde_json::from_str(EMBEDDED_PROFILE_JSON)
            .expect("embedded collection profile JSON must be valid")
    }

    /// Total number of individual collection items across all categories.
    pub fn total_items(&self) -> usize {
        self.logs.len()
            + self.registry.len()
            + self.event_logs.len()
            + self.exports.len()
            + self.commands.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_profile_deserializes() {
        let profile = CollectionProfile::embedded();
        assert_eq!(profile.profile_name, "cmtrace-full-diagnostics-v1");
        assert!(!profile.logs.is_empty());
        assert!(!profile.registry.is_empty());
        assert!(!profile.event_logs.is_empty());
        assert!(!profile.exports.is_empty());
        assert!(!profile.commands.is_empty());
    }

    #[test]
    fn total_items_sums_all_categories() {
        let profile = CollectionProfile::embedded();
        let expected = profile.logs.len()
            + profile.registry.len()
            + profile.event_logs.len()
            + profile.exports.len()
            + profile.commands.len();
        assert_eq!(profile.total_items(), expected);
        assert!(expected > 100, "full profile should have 100+ items");
    }
}

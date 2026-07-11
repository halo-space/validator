mod compile;
mod execute;
mod model;
mod validate;

pub(crate) use self::model::{Flow, Group};

#[cfg(test)]
mod tests {
    use super::model::Mode;

    #[test]
    fn modes_expose_only_their_declared_context() {
        assert!(!Mode::Value.fields());
        assert!(!Mode::Value.items());

        for mode in [Mode::Fields, Mode::FieldsWithAliases] {
            assert!(mode.fields());
            assert!(!mode.items());
        }

        for mode in [Mode::FieldsAndItems, Mode::FieldsAndItemsWithAliases] {
            assert!(mode.fields());
            assert!(mode.items());
        }
    }

    #[test]
    fn only_alias_modes_preserve_context_inside_aliases() {
        assert_eq!(Mode::Value.alias(), Mode::Value);
        assert_eq!(Mode::Fields.alias(), Mode::Value);
        assert_eq!(Mode::FieldsAndItems.alias(), Mode::Value);
        assert_eq!(Mode::FieldsWithAliases.alias(), Mode::FieldsWithAliases);
        assert_eq!(
            Mode::FieldsAndItemsWithAliases.alias(),
            Mode::FieldsAndItemsWithAliases
        );
    }
}

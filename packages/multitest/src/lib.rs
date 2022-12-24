// Testing utils. See tests folder for actual tests.

use cosmwasm_std::Attribute;

/// Gets the value of the first attribute with the given key
pub fn first_attr(data: impl AsRef<[Attribute]>, search_key: &str) -> Option<String> {
    data.as_ref().iter().find_map(|a| {
        if a.key == search_key {
            Some(a.value.clone())
        } else {
            None
        }
    })
}

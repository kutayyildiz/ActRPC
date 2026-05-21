#[macro_export]
macro_rules! action_descriptor_map {
    ($($action:ty),* $(,)?) => {{
        let mut map = ::std::collections::HashMap::new();

        $(
            let descriptor =
                <$action as $crate::action::ActionSpec>::descriptor();

            map.insert(descriptor.kind.clone(), descriptor);
        )*

        map
    }};
}

//! `[meta]` unset-fill: site_name into MetaDefaults.

use sova_core::App;
use sova_meta::{Meta, MetaDefaults};

#[tokio::test]
async fn meta_site_name_from_toml() {
    let mut app = App::new();
    app.configure_from_str(
        r#"
[meta]
site_name = "Toml Site"
"#,
    )
    .unwrap();
    app.install(Meta::new());

    let defaults = app.try_state::<MetaDefaults>().expect("MetaDefaults");
    assert_eq!(defaults.site_name.as_deref(), Some("Toml Site"));
}

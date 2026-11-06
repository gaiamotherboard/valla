//! The cookbook. Recipes are sample tools a business can start from; Valla adapts them on request.

pub struct Recipe {
    pub name: &'static str,
    pub about: &'static str,
    pub files: &'static [(&'static str, &'static str)],
}

pub const ALL: &[Recipe] = &[Recipe {
    name: "inventarium",
    about: "Inventory tracker: items, status, location, search. A small web app with no dependencies.",
    files: &[
        ("README.md", include_str!("../recipes/inventarium/README.md")),
        ("Cargo.toml", include_str!("../recipes/inventarium/Cargo.toml")),
        ("Cargo.lock", include_str!("../recipes/inventarium/Cargo.lock")),
        ("flake.nix", include_str!("../recipes/inventarium/flake.nix")),
        ("valla.sensors", include_str!("../recipes/inventarium/valla.sensors")),
        (".gitignore", include_str!("../recipes/inventarium/gitignore")),
        ("src/main.rs", include_str!("../recipes/inventarium/src/main.rs")),
        ("src/store.rs", include_str!("../recipes/inventarium/src/store.rs")),
        ("src/page.rs", include_str!("../recipes/inventarium/src/page.rs")),
    ],
}];

pub fn find(name: &str) -> Option<&'static Recipe> {
    ALL.iter().find(|r| r.name.eq_ignore_ascii_case(name))
}

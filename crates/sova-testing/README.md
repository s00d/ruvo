# sova-testing

Sqlite test DB, [`TestApp`] bootstrap, and response/snapshot helpers for Sova plugins.

Auth / notifications `acting_as` helpers stay in those plugins' own test utils (avoids a crates.io dependency cycle with this crate).

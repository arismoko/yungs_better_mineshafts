//! YUNG's Better Mineshafts — the one `impl Hooks` at the crate root.
//!
//! | Moment | What the port does |
//! |---|---|
//! | [`Mod::register`] | `register_structure_type("mineshaft", …)`: the type the jar's thirteen data-pack structures name (`"type": "bettermineshafts:mineshaft"`) now has a generator, so they stop being `Unsupported` |
//! | [`Mod::server_about_to_start`] | reads the settings (heights, spawn rates, small shaft length, ore deposits) the server's generation uses |
//! | [`Mod::register_commands`] | registers `/bettermineshafts status` and `/bettermineshafts last`, answered from [`runtime::status_lines`] and the last start's record |
//!
//! The mod's own geometry is [`pieces`] (the five piece kinds, saved and loaded like any structure
//! piece) over [`runtime`] (the pure layout: chain rules, boxes, clamps, the budget, the palette,
//! the readout), with [`support`] as the thin glue between them and the SDK.
//!
//! LOCAL-ONLY until publication is approved (ARCHITECTURE.md §8): LGPL-3.0-only, rank 1,
//! publishable under the same licence with this complete source.

pub mod pieces;
pub mod runtime;
pub mod support;

use mod_api::{
    self, Hooks, RegisterCommandsEvent, Registrar, RegistrationContext, ServerAboutToStartEvent,
    ServerHost,
};

#[derive(Default)]
struct Mod;

#[mod_api::sunrise_mod]
impl Hooks for Mod {
    /// Once, at startup: the structure type the carried data-pack definitions name.
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        support::register(r);
    }

    /// Before the server's worlds load: the settings this server's generation uses.
    fn server_about_to_start(&self, host: &mut dyn ServerHost, _: &mut ServerAboutToStartEvent) {
        support::read_settings(&*host);
    }

    /// Declare `/bettermineshafts status` in the server's command tree.
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        support::declare_commands(event.commands);
    }
}

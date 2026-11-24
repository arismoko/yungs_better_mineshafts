//! YUNG's Better Mineshafts — the one `impl Hooks` at the crate root.
//!
//! | Moment | What the port does |
//! |---|---|
//! | [`Mod::register`] | `register_structure_type("mineshaft", …)`: the type the jar's thirteen data-pack structures name (`"type": "bettermineshafts:mineshaft"`) now has a generator, so they stop being `Unsupported` |
//! | [`Mod::server_about_to_start`] | reads the settings (heights, spawn rates, small shaft length, ore deposits) the server's generation uses |
//! | [`Mod::register_commands`] | registers `/bettermineshafts status` and `/bettermineshafts last`, answered from [`runtime::status_lines`] and the last start's record |
//! | [`Mod::player_command`] | answers `/locate structure mineshaft` with the original's hint, as vanilla mineshafts are off |
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
    self, Flow, Hooks, PlayerCommandEvent, RegisterCommandsEvent, Registrar, RegistrationContext,
    ServerAboutToStartEvent, ServerHost, Text,
};

/// `LocateVanillaMineshaftCommandMixin.VANILLA_MINESHAFT_EXCEPTION`.
const LOCATE_HINT: &str = "Use /locate structure #bettermineshafts:better_mineshafts instead!";

/// Whether `command` (without its slash) locates the vanilla mineshaft by id, the case the
/// original's `LocateCommand.locateStructure` injection refuses; a `#tag` and
/// `minecraft:mineshaft_mesa` are not that case.
fn locates_vanilla_mineshaft(command: &str) -> bool {
    let mut words = command.split_whitespace();
    matches!(
        (words.next(), words.next(), words.next(), words.next()),
        (
            Some("locate"),
            Some("structure"),
            Some("mineshaft" | "minecraft:mineshaft"),
            None
        )
    )
}

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

    /// The original refuses `/locate structure minecraft:mineshaft` while vanilla mineshafts are
    /// off (always, here) with a hint to the mod's tag. Typed commands only: the API has no hook
    /// at `LocateCommand.locateStructure` itself (sunrise#5266).
    fn player_command(
        &self,
        _host: &mut dyn ServerHost,
        event: &mut PlayerCommandEvent<'_>,
    ) -> Flow {
        if !locates_vanilla_mineshaft(event.command) {
            return Flow::Continue;
        }
        // A command exception shows in red (`CommandSourceStack.sendFailure`).
        let mut message = Text::literal(LOCATE_HINT);
        message.style.color = Some("red".to_owned());
        event.replies.push(message);
        Flow::Cancel
    }
}

#[cfg(test)]
mod tests {
    use super::locates_vanilla_mineshaft;

    #[test]
    fn only_the_vanilla_mineshaft_by_id_gets_the_hint() {
        assert!(locates_vanilla_mineshaft("locate structure mineshaft"));
        assert!(locates_vanilla_mineshaft(
            "locate structure minecraft:mineshaft"
        ));
        assert!(!locates_vanilla_mineshaft(
            "locate structure minecraft:mineshaft_mesa"
        ));
        assert!(!locates_vanilla_mineshaft(
            "locate structure #bettermineshafts:better_mineshafts"
        ));
        assert!(!locates_vanilla_mineshaft("locate biome mineshaft"));
    }
}

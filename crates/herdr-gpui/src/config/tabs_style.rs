//! The `[tabs]` table beyond its font: what each tab shows and where the tab
//! strips sit.

use super::fonts::FontSettings;
use serde::Deserialize;

/// What a tab shows besides its name.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TabContent {
    /// Only the name.
    #[default]
    Name,
    /// The name and its agent's logo, or the terminal icon.
    Agent,
    /// As `agent`, and the agent's state badge.
    Status,
}

/// Where the tab strips sit.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TabPlacement {
    /// Under the main header, whatever Herdr's tab bar position says.
    #[default]
    Subheader,
    /// At the foot of the content, above the status bar.
    Bottom,
    /// Where Herdr's `tab_bar_position` puts them.
    Herdr,
}

/// Tab styling the config file sets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TabsStyle {
    pub content: TabContent,
    /// Read from the `position` key.
    pub placement: TabPlacement,
}

/// The raw `[tabs]` table: font keys beside the style keys. Deserialized by
/// hand, as `[sidebar]` is, so a key this build does not know still reaches
/// the ignored key reporter.
#[derive(Default)]
pub(super) struct TabsSettings {
    pub(super) font: FontSettings,
    pub(super) style: TabsStyle,
}

impl<'de> Deserialize<'de> for TabsSettings {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = TabsSettings;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a [tabs] table")
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<TabsSettings, A::Error> {
                let mut font = toml::Table::new();
                let mut settings = TabsSettings::default();
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        // The terminal-only keys go with the font so they are
                        // reported as they were before this table had style keys.
                        "family" | "size" | "fallback" | "line_height" | "weight"
                        | "padding_x" | "padding_y" => {
                            font.insert(key, map.next_value()?);
                        }
                        "content" => settings.style.content = map.next_value()?,
                        "position" => settings.style.placement = map.next_value()?,
                        _ => {
                            map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }
                settings.font = FontSettings::deserialize(toml::Value::Table(font))
                    .map_err(serde::de::Error::custom)?;
                Ok(settings)
            }
        }

        deserializer.deserialize_map(Visitor)
    }
}

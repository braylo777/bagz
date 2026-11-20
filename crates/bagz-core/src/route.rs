//! Hash routes for the five demo screens.
//!
//! Aliases match `src/routes.ts`: `deck` is the swipe screen, `agent` is chat,
//! and `tiers` is profile. Anything else, including an empty hash, is splash.

/// A screen in the demo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// Opening screen.
    Splash,
    /// Swipe deck.
    Bagz,
    /// Agent timeline.
    Chat,
    /// Seeker profile.
    Profile,
    /// Demo checkout.
    Checkout,
}

impl Route {
    /// Canonical hash path without the `#/`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Splash => "splash",
            Self::Bagz => "bagz",
            Self::Chat => "chat",
            Self::Profile => "profile",
            Self::Checkout => "checkout",
        }
    }
}

/// Resolve a location hash to a screen.
///
/// The leading `#` and one optional `/` are stripped, matching the
/// `^#/?` replace in the demo. A path that does not start with `#` is
/// matched as-is, so `bagz` works and `/bagz` does not.
#[must_use]
pub fn parse_route(hash: &str) -> Route {
    let path = match hash.strip_prefix('#') {
        Some(rest) => rest.strip_prefix('/').unwrap_or(rest),
        None => hash,
    };
    match path {
        "splash" => Route::Splash,
        "bagz" | "deck" => Route::Bagz,
        "chat" | "agent" => Route::Chat,
        "profile" | "tiers" => Route::Profile,
        "checkout" => Route::Checkout,
        _ => Route::Splash,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_and_unknown_paths() {
        assert_eq!(parse_route("#/bagz"), Route::Bagz);
        assert_eq!(parse_route("#bagz"), Route::Bagz);
        assert_eq!(parse_route("bagz"), Route::Bagz);
        assert_eq!(parse_route("#/deck"), Route::Bagz);
        assert_eq!(parse_route("#/agent"), Route::Chat);
        assert_eq!(parse_route("#/tiers"), Route::Profile);
        assert_eq!(parse_route("#/checkout"), Route::Checkout);
        assert_eq!(parse_route("#/splash"), Route::Splash);
        assert_eq!(parse_route(""), Route::Splash);
        assert_eq!(parse_route("#/"), Route::Splash);
        assert_eq!(parse_route("#//bagz"), Route::Splash);
        assert_eq!(parse_route("/bagz"), Route::Splash);
        assert_eq!(parse_route("#/nope"), Route::Splash);
        assert_eq!(Route::Chat.as_str(), "chat");
    }
}

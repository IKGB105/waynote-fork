/// Colour tokens for each note colour variant.
///
/// All hex strings are 7-char `#RRGGBB`.  `code_bg` uses `rgba(…)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub bg: &'static str,
    pub ink: &'static str,
    pub muted: &'static str,
    pub accent: &'static str,
    pub code_bg: &'static str,
    pub border: &'static str,
}

const YELLOW: Theme = Theme {
    bg: "#F6EBA8", ink: "#403A28", muted: "#8A7F5E",
    accent: "#B07A1E", code_bg: "rgba(0,0,0,0.06)", border: "#E3D58C",
};
const GREEN: Theme = Theme {
    bg: "#CDE8C5", ink: "#283322", muted: "#5E7355",
    accent: "#3E7A3E", code_bg: "rgba(0,0,0,0.06)", border: "#B8D9AE",
};
const BLUE: Theme = Theme {
    bg: "#C7DDF1", ink: "#243240", muted: "#5A6E80",
    accent: "#2C6FA8", code_bg: "rgba(0,0,0,0.06)", border: "#AECBE6",
};
const PINK: Theme = Theme {
    bg: "#F4D2DA", ink: "#3C2630", muted: "#836670",
    accent: "#B0506E", code_bg: "rgba(0,0,0,0.06)", border: "#E6BAC5",
};
const PURPLE: Theme = Theme {
    bg: "#E0D4F0", ink: "#322A40", muted: "#6E6383",
    accent: "#6B4FA0", code_bg: "rgba(0,0,0,0.06)", border: "#CBBCE6",
};
const GRAY: Theme = Theme {
    bg: "#DEDCD6", ink: "#33322E", muted: "#73726C",
    accent: "#5E5C55", code_bg: "rgba(0,0,0,0.06)", border: "#C9C7C0",
};
const ORANGE: Theme = Theme {
    bg: "#F6D9B8", ink: "#43321F", muted: "#8A7152",
    accent: "#BD6415", code_bg: "rgba(0,0,0,0.06)", border: "#E8C49A",
};
const RED: Theme = Theme {
    bg: "#F6C9C2", ink: "#402420", muted: "#8A625C",
    accent: "#B23A2E", code_bg: "rgba(0,0,0,0.06)", border: "#E8ADA3",
};
const TEAL: Theme = Theme {
    bg: "#BFE5DE", ink: "#223836", muted: "#557670",
    accent: "#1F7A6C", code_bg: "rgba(0,0,0,0.06)", border: "#A3D4CA",
};
const BROWN: Theme = Theme {
    bg: "#E3CBB0", ink: "#3A2A1C", muted: "#7D6650",
    accent: "#8B5A2B", code_bg: "rgba(0,0,0,0.06)", border: "#D0B08C",
};

/// The note colour palette, in picker order. Single source of truth for the valid
/// colour set (used by the picker UI, `set_color`, and `on_color_requested`).
pub const NOTE_COLORS: [&str; 10] =
    ["yellow", "green", "blue", "pink", "purple", "gray", "orange", "red", "teal", "brown"];

/// Whether `color` is one of the known palette colours.
pub fn is_note_color(color: &str) -> bool {
    NOTE_COLORS.contains(&color)
}

/// PURE: pick a palette colour for a new note, varying from note to note
/// without an RNG dependency - every note id is unique (ULID), so summing
/// its bytes gives a value that's effectively random across different notes
/// while staying deterministic (and trivially testable) for a given id.
pub fn random_color(id: &str) -> &'static str {
    let sum: u32 = id.bytes().map(u32::from).sum();
    NOTE_COLORS[(sum as usize) % NOTE_COLORS.len()]
}

/// Return the colour tokens for `color`. Unknown names fall back to yellow.
pub fn theme(color: &str) -> Theme {
    match color {
        "yellow" => YELLOW,
        "green"  => GREEN,
        "blue"   => BLUE,
        "pink"   => PINK,
        "purple" => PURPLE,
        "gray"   => GRAY,
        "orange" => ORANGE,
        "red"    => RED,
        "teal"   => TEAL,
        "brown"  => BROWN,
        _        => YELLOW,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blue_bg_is_correct() {
        assert_eq!(theme("blue").bg, "#C7DDF1");
    }

    #[test]
    fn unknown_color_falls_back_to_yellow() {
        assert_eq!(theme("mauve"), theme("yellow"));
    }

    #[test]
    fn random_color_is_always_a_valid_palette_colour() {
        for id in ["01A", "01KZQXSG7T6XZR9VTVDTXGP2KY", "", "z"] {
            assert!(is_note_color(random_color(id)), "{id} produced an invalid colour");
        }
    }

    #[test]
    fn random_color_is_deterministic_for_the_same_id() {
        assert_eq!(random_color("01KZQXSG7T6XZR9VTVDTXGP2KY"), random_color("01KZQXSG7T6XZR9VTVDTXGP2KY"));
    }

    #[test]
    fn random_color_varies_across_different_ids() {
        // Not a proof of "randomness", just a smoke test that it isn't
        // hardcoded to always return the same colour.
        let colors: std::collections::HashSet<_> =
            (0..20u32).map(|i| random_color(&format!("01KZQXSG7T6XZR9VTVDTXGP{i:02}"))).collect();
        assert!(colors.len() > 1, "20 different ids all produced the same colour");
    }

    const ALL_COLORS: [&str; 10] =
        ["yellow", "green", "blue", "pink", "purple", "gray", "orange", "red", "teal", "brown"];

    #[test]
    fn all_ten_bg_pairwise_distinct() {
        let bgs: Vec<_> = ALL_COLORS.iter().map(|c| theme(c).bg).collect();
        for i in 0..bgs.len() {
            for j in (i + 1)..bgs.len() {
                assert_ne!(bgs[i], bgs[j], "bg for {i} and {j} must differ");
            }
        }
    }

    #[test]
    fn all_ten_ink_pairwise_distinct() {
        let inks: Vec<_> = ALL_COLORS.iter().map(|c| theme(c).ink).collect();
        for i in 0..inks.len() {
            for j in (i + 1)..inks.len() {
                assert_ne!(inks[i], inks[j], "ink for {i} and {j} must differ");
            }
        }
    }

    #[test]
    fn all_ten_accent_pairwise_distinct() {
        let accents: Vec<_> = ALL_COLORS.iter().map(|c| theme(c).accent).collect();
        for i in 0..accents.len() {
            for j in (i + 1)..accents.len() {
                assert_ne!(accents[i], accents[j], "accent for {i} and {j} must differ");
            }
        }
    }

    fn is_hex7(s: &str) -> bool {
        s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
    }

    #[test]
    fn hex_fields_are_7_char_hash_hex() {
        for color in &ALL_COLORS {
            let t = theme(color);
            assert!(is_hex7(t.bg),     "{color}: bg not 7-char hex");
            assert!(is_hex7(t.ink),    "{color}: ink not 7-char hex");
            assert!(is_hex7(t.muted),  "{color}: muted not 7-char hex");
            assert!(is_hex7(t.accent), "{color}: accent not 7-char hex");
            assert!(is_hex7(t.border), "{color}: border not 7-char hex");
        }
    }
}

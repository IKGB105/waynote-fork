//! Auto-arrange: lay out notes into a deterministic left-to-right, top-to-bottom
//! flow within a surface's bounds. PURE — no GTK, no I/O.
//!
//! Row-major order: notes fill a row left-to-right, then wrap to the next row
//! when they'd overflow the surface width. Each note keeps its OWN current
//! size — arrange only repositions, it never resizes.

use crate::platform::geometry::Rect;

use super::note_entry::NoteId;

/// PURE: assign flow positions to `ids` (in given order) within `bounds`,
/// keeping each note's own size from `sizes` (same order, same length as
/// `ids`).
///
/// - `sizes`: (width, height) of each note, in the same order as `ids`.
/// - `margin`: (left, top) margin inside `bounds`.
/// - `gap`: gap between notes (both axes).
///
/// A row fills left-to-right; a note that would cross `bounds`'s right edge
/// wraps to a new row below the tallest note seen so far in the current row.
/// The very first note in a row is never wrapped (so an oversized note still
/// gets a deterministic slot instead of looping forever).
pub fn arrange_grid(
    ids: &[NoteId],
    sizes: &[(i32, i32)],
    bounds: Rect,
    margin: (i32, i32),
    gap: i32,
) -> Vec<(NoteId, Rect)> {
    let right_edge = bounds.w;
    let mut x = margin.0;
    let mut y = margin.1;
    let mut row_h = 0;

    ids.iter()
        .zip(sizes.iter())
        .map(|(id, &(w, h))| {
            if x != margin.0 && x + w > right_edge {
                x = margin.0;
                y += row_h + gap;
                row_h = 0;
            }
            let r = Rect { x: bounds.x + x, y: bounds.y + y, w, h };
            x += w + gap;
            row_h = row_h.max(h);
            (id.clone(), r)
        })
        .collect()
}

/// PURE: where a new note of `new_size` should land next in the same
/// left-to-right, top-to-bottom flow as `arrange_grid`, WITHOUT moving any
/// existing note. `existing_sizes` is every other note currently on the
/// surface, in the same order `arrange_grid`'s `ids` would use (sorted by
/// id) — their real (possibly manually-dragged-away) positions are ignored
/// on purpose; this only simulates "if everything so far had been laid out
/// in flow order, where would one more note go" and returns just that last
/// slot.
pub fn next_flow_position(
    existing_sizes: &[(i32, i32)],
    new_size: (i32, i32),
    bounds: Rect,
    margin: (i32, i32),
    gap: i32,
) -> Rect {
    let ids: Vec<NoteId> = (0..=existing_sizes.len()).map(|i| i.to_string()).collect();
    let sizes: Vec<(i32, i32)> =
        existing_sizes.iter().copied().chain(std::iter::once(new_size)).collect();
    arrange_grid(&ids, &sizes, bounds, margin, gap)
        .pop()
        .map(|(_, r)| r)
        .unwrap_or(Rect { x: bounds.x + margin.0, y: bounds.y + margin.1, w: new_size.0, h: new_size.1 })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(names: &[&str]) -> Vec<NoteId> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn next_flow_position_with_no_existing_notes_starts_at_the_margin() {
        let bounds = Rect { x: 0, y: 0, w: 1920, h: 1080 };
        let r = next_flow_position(&[], (240, 200), bounds, (24, 48), 16);
        assert_eq!(r, Rect { x: 24, y: 48, w: 240, h: 200 });
    }

    #[test]
    fn next_flow_position_continues_the_row_after_existing_notes() {
        let bounds = Rect { x: 0, y: 0, w: 1920, h: 1080 };
        let r = next_flow_position(&[(240, 200)], (240, 200), bounds, (24, 48), 16);
        assert_eq!(r, Rect { x: 24 + 240 + 16, y: 48, w: 240, h: 200 });
    }

    #[test]
    fn next_flow_position_wraps_when_the_row_is_full() {
        // Bounds w=500: one 240-wide existing note leaves no room for another
        // 240-wide one on the same row (240+240=480 fits, but margin+gap eats it).
        let bounds = Rect { x: 0, y: 0, w: 500, h: 1080 };
        let r = next_flow_position(&[(240, 200), (240, 100)], (240, 150), bounds, (0, 0), 0);
        assert_eq!(r.x, 0, "wraps back to the left edge");
        assert_eq!(r.y, 200, "starts below the tallest note in the previous row");
    }

    #[test]
    fn arrange_grid_lays_notes_left_to_right_in_a_row() {
        let ids = ids(&["a", "b", "c"]);
        let sizes = [(240, 200), (240, 200), (240, 200)];
        let bounds = Rect { x: 0, y: 0, w: 1920, h: 1080 };
        let out = arrange_grid(&ids, &sizes, bounds, (24, 48), 16);

        assert_eq!(out.len(), 3);
        assert_eq!(out[0].1, Rect { x: 24, y: 48, w: 240, h: 200 });
        // second note is to the right of the first, same row
        assert_eq!(out[1].1.y, out[0].1.y, "second note is in the same row");
        assert_eq!(out[1].1.x, out[0].1.x + 240 + 16, "second note is to the right of the first");
        assert_eq!(out[2].1.x, out[1].1.x + 240 + 16, "third note follows the second");
    }

    #[test]
    fn arrange_grid_preserves_each_notes_own_size() {
        let ids = ids(&["a", "b"]);
        let sizes = [(400, 150), (180, 260)];
        let bounds = Rect { x: 0, y: 0, w: 1920, h: 1080 };
        let out = arrange_grid(&ids, &sizes, bounds, (0, 0), 0);

        assert_eq!(out[0].1.w, 400, "note keeps its own width");
        assert_eq!(out[0].1.h, 150, "note keeps its own height");
        assert_eq!(out[1].1.w, 180, "second note keeps its own width");
        assert_eq!(out[1].1.h, 260, "second note keeps its own height");
    }

    #[test]
    fn arrange_grid_wraps_to_next_row_when_row_is_full() {
        // Bounds w=500: two 240-wide notes (240+240=480) fit, a third doesn't.
        let ids = ids(&["a", "b", "c"]);
        let sizes = [(240, 200), (240, 100), (240, 150)];
        let bounds = Rect { x: 0, y: 0, w: 500, h: 1080 };
        let out = arrange_grid(&ids, &sizes, bounds, (0, 0), 0);

        assert_eq!(out[0].1, Rect { x: 0, y: 0, w: 240, h: 200 });
        assert_eq!(out[1].1, Rect { x: 240, y: 0, w: 240, h: 100 });
        // third note wraps: starts a new row below the TALLEST note in row 0 (200, not 100)
        assert_eq!(out[2].1.x, 0, "third note wraps back to the left edge");
        assert_eq!(out[2].1.y, 200, "third note starts below the tallest note in the previous row");
    }

    #[test]
    fn arrange_grid_oversized_note_still_gets_a_slot() {
        // A note wider than bounds must not wrap against itself (infinite/empty row).
        let ids = ids(&["a", "b"]);
        let sizes = [(2000, 200), (240, 200)];
        let bounds = Rect { x: 0, y: 0, w: 500, h: 1080 };
        let out = arrange_grid(&ids, &sizes, bounds, (0, 0), 16);

        assert_eq!(out[0].1, Rect { x: 0, y: 0, w: 2000, h: 200 });
        // second note wraps to its own row rather than sitting inside the first note
        assert_eq!(out[1].1.x, 0);
        assert_eq!(out[1].1.y, 200 + 16);
    }

    #[test]
    fn arrange_grid_empty_ids_returns_empty() {
        let out = arrange_grid(&[], &[], Rect { x: 0, y: 0, w: 1920, h: 1080 }, (24, 48), 16);
        assert!(out.is_empty());
    }

    #[test]
    fn arrange_grid_output_ids_match_input_order() {
        let ids = ids(&["z", "m", "a"]);
        let sizes = [(240, 200), (240, 200), (240, 200)];
        let out = arrange_grid(&ids, &sizes, Rect { x: 0, y: 0, w: 1920, h: 1080 }, (0, 0), 8);
        assert_eq!(out.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(), vec!["z", "m", "a"]);
    }
}

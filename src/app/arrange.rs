//! Auto-arrange: lay out notes into a deterministic left-to-right, top-to-bottom
//! flow within a surface's bounds. PURE — no GTK, no I/O.
//!
//! Two flow orders: `arrange_grid` fills a row left-to-right, wrapping to the
//! next row when it would overflow the surface width (used for single-note
//! placement — a new note, a restored note's overlap-displacement target).
//! `arrange_blocks` groups notes into fixed 2×2 blocks instead (used by the
//! "Arrange" action) — both keep each note's OWN current size; arrange only
//! repositions, it never resizes.

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

/// Total width of up to 2 items placed side by side with `gap` between them —
/// the "row span" a 2×2 block's top or bottom half occupies.
fn row_span_width(items: &[(i32, i32)], gap: i32) -> i32 {
    match items {
        [] => 0,
        [(w, _)] => *w,
        [(w0, _), (w1, _), ..] => w0 + gap + w1,
    }
}

/// PURE: like `arrange_grid`, but groups `ids` into fixed 2×2 blocks (top-left,
/// top-right, bottom-left, bottom-right, taken 4 at a time in `ids`' order)
/// instead of one width-driven row flow. A block fills both its own rows
/// before the next block starts, immediately to its right; blocks wrap to a
/// new row of blocks using the same rule `arrange_grid` uses for notes — the
/// first block in a row-of-blocks is never wrapped, so an oversized block
/// still gets a deterministic slot. A trailing partial block (id count not a
/// multiple of 4) just has fewer of its 4 slots filled — 3 notes make an
/// "L" (top-left, top-right, bottom-left), 2 make a top row, 1 sits alone.
pub fn arrange_blocks(
    ids: &[NoteId],
    sizes: &[(i32, i32)],
    bounds: Rect,
    margin: (i32, i32),
    gap: i32,
) -> Vec<(NoteId, Rect)> {
    let right_edge = bounds.w;
    let mut block_x = margin.0;
    let mut y = margin.1;
    let mut row_of_blocks_h = 0;
    let mut out = Vec::with_capacity(ids.len());

    for (chunk_ids, chunk_sizes) in ids.chunks(4).zip(sizes.chunks(4)) {
        let top = &chunk_sizes[..chunk_sizes.len().min(2)];
        let bottom = if chunk_sizes.len() > 2 { &chunk_sizes[2..] } else { &[] };
        let top_h = top.iter().map(|&(_, h)| h).max().unwrap_or(0);
        let bottom_h = bottom.iter().map(|&(_, h)| h).max().unwrap_or(0);
        let block_w = row_span_width(top, gap).max(row_span_width(bottom, gap));
        let block_h = if bottom.is_empty() { top_h } else { top_h + gap + bottom_h };

        if block_x != margin.0 && block_x + block_w > right_edge {
            block_x = margin.0;
            y += row_of_blocks_h + gap;
            row_of_blocks_h = 0;
        }

        let mut place_row = |row_ids: &[NoteId], row_sizes: &[(i32, i32)], row_y: i32| {
            let mut x = block_x;
            for (id, &(w, h)) in row_ids.iter().zip(row_sizes.iter()) {
                out.push((id.clone(), Rect { x: bounds.x + x, y: bounds.y + row_y, w, h }));
                x += w + gap;
            }
        };
        place_row(&chunk_ids[..chunk_ids.len().min(2)], top, y);
        if chunk_ids.len() > 2 {
            place_row(&chunk_ids[2..], bottom, y + top_h + gap);
        }

        block_x += block_w + gap;
        row_of_blocks_h = row_of_blocks_h.max(block_h);
    }

    out
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

    #[test]
    fn arrange_blocks_fills_a_2x2_block_top_row_then_bottom_row() {
        // Exactly the order asked for: 1,2 on top, 3,4 below them, in ONE
        // block, before a second block starts.
        let ids = ids(&["1", "2", "3", "4"]);
        let sizes = [(200, 100); 4];
        let bounds = Rect { x: 0, y: 0, w: 1920, h: 1080 };
        let out = arrange_blocks(&ids, &sizes, bounds, (0, 0), 10);

        assert_eq!(out[0].1, Rect { x: 0, y: 0, w: 200, h: 100 }, "1: top-left");
        assert_eq!(out[1].1, Rect { x: 210, y: 0, w: 200, h: 100 }, "2: top-right");
        assert_eq!(out[2].1, Rect { x: 0, y: 110, w: 200, h: 100 }, "3: bottom-left");
        assert_eq!(out[3].1, Rect { x: 210, y: 110, w: 200, h: 100 }, "4: bottom-right");
    }

    #[test]
    fn arrange_blocks_starts_the_next_block_to_the_right_after_both_rows() {
        // 8 notes, 2 blocks: 5-8 must start a fresh block to the right of
        // 1-4, back at the top row — not continue row 0 or row 1 directly.
        let ids = ids(&["1", "2", "3", "4", "5", "6", "7", "8"]);
        let sizes = [(200, 100); 8];
        let bounds = Rect { x: 0, y: 0, w: 1920, h: 1080 };
        let out = arrange_blocks(&ids, &sizes, bounds, (0, 0), 10);

        assert_eq!(out[4].1, Rect { x: 420, y: 0, w: 200, h: 100 }, "5: top-left of block 2");
        assert_eq!(out[5].1, Rect { x: 630, y: 0, w: 200, h: 100 }, "6: top-right of block 2");
        assert_eq!(out[6].1, Rect { x: 420, y: 110, w: 200, h: 100 }, "7: bottom-left of block 2");
        assert_eq!(out[7].1, Rect { x: 630, y: 110, w: 200, h: 100 }, "8: bottom-right of block 2");
    }

    #[test]
    fn arrange_blocks_wraps_to_a_new_row_of_blocks_when_it_overflows() {
        // Bounds only wide enough for ONE block (2×200 + gap = 410); a second
        // block must wrap below, not spill past the right edge.
        let ids = ids(&["1", "2", "3", "4", "5", "6", "7", "8"]);
        let sizes = [(200, 100); 8];
        let bounds = Rect { x: 0, y: 0, w: 420, h: 1080 };
        let out = arrange_blocks(&ids, &sizes, bounds, (0, 0), 10);

        assert_eq!(out[4].1.x, 0, "second block wraps back to the left edge");
        assert_eq!(out[4].1.y, 220, "second block starts below the first block's full height (100+10+100)");
    }

    #[test]
    fn arrange_blocks_trailing_partial_block_only_fills_what_it_has() {
        // 3 notes: top-left, top-right, bottom-left — no bottom-right.
        let ids = ids(&["1", "2", "3"]);
        let sizes = [(200, 100); 3];
        let bounds = Rect { x: 0, y: 0, w: 1920, h: 1080 };
        let out = arrange_blocks(&ids, &sizes, bounds, (0, 0), 10);

        assert_eq!(out.len(), 3);
        assert_eq!(out[2].1, Rect { x: 0, y: 110, w: 200, h: 100 }, "3: bottom-left, alone in row 1");
    }

    #[test]
    fn arrange_blocks_preserves_each_notes_own_size() {
        let ids = ids(&["1", "2", "3", "4"]);
        let sizes = [(400, 150), (180, 260), (220, 90), (300, 200)];
        let bounds = Rect { x: 0, y: 0, w: 1920, h: 1080 };
        let out = arrange_blocks(&ids, &sizes, bounds, (0, 0), 10);

        for (i, &(w, h)) in sizes.iter().enumerate() {
            assert_eq!(out[i].1.w, w, "note {i} keeps its own width");
            assert_eq!(out[i].1.h, h, "note {i} keeps its own height");
        }
    }
}

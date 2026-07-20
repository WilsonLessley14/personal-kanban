/// Compute the next position value, placing the new item after all existing items.
///
/// The next position is `max(existing) + 1`, or `0` if the list is empty.
pub fn next_position(existing_positions: &[i32]) -> i32 {
    if existing_positions.is_empty() {
        return 0;
    }
    existing_positions.iter().copied().max().unwrap() + 1
}

/// Recompute positions to be gap-free 0, 1, 2, … after deletes or moves.
///
/// The items are sorted by their current position and renumbered sequentially.
pub fn recompute_positions(items: &[(String, i32)]) -> Vec<(String, i32)> {
    let mut sorted = items.to_vec();
    sorted.sort_by_key(|&(_, pos)| pos);
    sorted
        .into_iter()
        .enumerate()
        .map(|(i, (id, _))| (id, i as i32))
        .collect()
}

/// Compute new positions after inserting an item at a given position.
///
/// Items at the insertion point and after are shifted by +1.
pub fn positions_after_insert(existing: &[(String, i32)], insert_at: i32) -> Vec<(String, i32)> {
    let mut result = Vec::with_capacity(existing.len());
    for (id, pos) in existing {
        let new_pos = if *pos >= insert_at { pos + 1 } else { *pos };
        result.push((id.clone(), new_pos));
    }
    result
}

/// Compute new positions after moving an item from one position to another.
///
/// Items between the old and new position are shifted to close the gap.
pub fn positions_after_move(existing: &[(String, i32)], from: i32, to: i32) -> Vec<(String, i32)> {
    let mut result = Vec::with_capacity(existing.len());
    for (id, pos) in existing {
        let new_pos = match (*pos, from <= to) {
            // Moving forward: items between from+1..to shift down by 1
            (_, true) if *pos == from => to,
            (_, true) if *pos > from && *pos <= to => pos - 1,
            // Moving backward: items between to..from-1 shift up by 1
            (_, false) if *pos == from => to,
            (_, false) if *pos >= to && *pos < from => pos + 1,
            _ => *pos,
        };
        result.push((id.clone(), new_pos));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_position_empty() {
        assert_eq!(next_position(&[]), 0);
    }

    #[test]
    fn next_position_single() {
        assert_eq!(next_position(&[0]), 1);
    }

    #[test]
    fn next_position_gapped() {
        assert_eq!(next_position(&[0, 2, 5]), 6);
    }

    #[test]
    fn next_position_negative() {
        assert_eq!(next_position(&[-5, -2]), -1);
    }

    #[test]
    fn recompute_positions_empty() {
        assert!(recompute_positions(&[]).is_empty());
    }

    #[test]
    fn recompute_positions_sequential() {
        let items = vec![("a".into(), 0), ("b".into(), 1), ("c".into(), 2)];
        let result = recompute_positions(&items);
        assert_eq!(
            result,
            vec![("a".into(), 0), ("b".into(), 1), ("c".into(), 2)]
        );
    }

    #[test]
    fn recompute_positions_gaps() {
        let items = vec![("a".into(), 0), ("c".into(), 5)];
        let result = recompute_positions(&items);
        assert_eq!(result, vec![("a".into(), 0), ("c".into(), 1)]);
    }

    #[test]
    fn recompute_positions_unsorted() {
        let items = vec![("b".into(), 2), ("a".into(), 0), ("c".into(), 1)];
        let result = recompute_positions(&items);
        assert_eq!(
            result,
            vec![("a".into(), 0), ("c".into(), 1), ("b".into(), 2)]
        );
    }

    #[test]
    fn positions_after_insert_at_end() {
        let existing = vec![("a".into(), 0), ("b".into(), 1)];
        let result = positions_after_insert(&existing, 2);
        // Items unchanged, new item would be at 2
        assert_eq!(result, vec![("a".into(), 0), ("b".into(), 1)]);
    }

    #[test]
    fn positions_after_insert_at_beginning() {
        let existing = vec![("a".into(), 0), ("b".into(), 1)];
        let result = positions_after_insert(&existing, 0);
        assert_eq!(result, vec![("a".into(), 1), ("b".into(), 2)]);
    }

    #[test]
    fn positions_after_insert_in_middle() {
        let existing = vec![("a".into(), 0), ("b".into(), 1), ("c".into(), 2)];
        let result = positions_after_insert(&existing, 1);
        assert_eq!(
            result,
            vec![("a".into(), 0), ("b".into(), 2), ("c".into(), 3)]
        );
    }

    #[test]
    fn positions_after_move_forward() {
        let existing = vec![("a".into(), 0), ("b".into(), 1), ("c".into(), 2)];
        let result = positions_after_move(&existing, 0, 2);
        // a moves from 0→2, b shifts 1→0, c shifts 2→1
        assert_eq!(
            result,
            vec![("a".into(), 2), ("b".into(), 0), ("c".into(), 1)]
        );
    }

    #[test]
    fn positions_after_move_backward() {
        let existing = vec![("a".into(), 0), ("b".into(), 1), ("c".into(), 2)];
        let result = positions_after_move(&existing, 2, 0);
        // c moves from 2→0, a shifts 0→1, b shifts 1→2
        assert_eq!(
            result,
            vec![("a".into(), 1), ("b".into(), 2), ("c".into(), 0)]
        );
    }

    #[test]
    fn positions_after_move_same_position() {
        let existing = vec![("a".into(), 0), ("b".into(), 1)];
        let result = positions_after_move(&existing, 0, 0);
        assert_eq!(result, vec![("a".into(), 0), ("b".into(), 1)]);
    }

    #[test]
    fn positions_after_move_adjacent_forward() {
        let existing = vec![("a".into(), 0), ("b".into(), 1)];
        let result = positions_after_move(&existing, 0, 1);
        // a moves from 0→1, b shifts 1→0
        assert_eq!(result, vec![("a".into(), 1), ("b".into(), 0)]);
    }

    #[test]
    fn positions_after_move_adjacent_backward() {
        let existing = vec![("a".into(), 0), ("b".into(), 1)];
        let result = positions_after_move(&existing, 1, 0);
        // b moves from 1→0, a shifts 0→1
        assert_eq!(result, vec![("a".into(), 1), ("b".into(), 0)]);
    }
}

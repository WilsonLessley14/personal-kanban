use crate::core::error::IdError;

/// Resolve a short ID prefix to a full ID.
///
/// Returns the matched full ID if exactly one candidate starts with the prefix,
/// `IdError::Ambiguous` if multiple match, or `IdError::NotFound` if none match.
pub fn resolve_id(prefix: &str, candidates: &[&str]) -> Result<String, IdError> {
    let matches: Vec<&str> = candidates
        .iter()
        .copied()
        .filter(|id| id.starts_with(prefix))
        .collect();

    match matches.len() {
        0 => Err(IdError::NotFound {
            prefix: prefix.to_string(),
        }),
        1 => Ok(matches[0].to_string()),
        _ => Err(IdError::Ambiguous {
            prefix: prefix.to_string(),
            matches: matches.iter().map(|s| s.to_string()).collect(),
        }),
    }
}

/// Compute the minimum unique prefix length for each ID in the set.
///
/// For each ID, find the shortest prefix that distinguishes it from every other ID.
/// Returns a vector of `(id, min_prefix_length)` pairs.
pub fn min_unique_prefixes(ids: &[&str]) -> Vec<(String, usize)> {
    if ids.is_empty() {
        return vec![];
    }
    ids.iter()
        .map(|&id| {
            let min_len = ids
                .iter()
                .filter(|&&other| other != id)
                .map(|&other| {
                    // Find the first position where id and other differ.
                    // The minimum prefix must extend at least that far.
                    let diff_pos = id
                        .chars()
                        .zip(other.chars())
                        .position(|(a, b)| a != b)
                        .unwrap_or(id.len());
                    diff_pos + 1
                })
                .max()
                .unwrap_or(1) // If it's the only ID, minimum prefix is 1 char
                .min(id.len());
            (id.to_string(), min_len)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_id_exact_match() {
        let ids = vec!["a3x9k2", "a3bQ7f", "m8rT2p"];
        assert_eq!(resolve_id("a3x9k2", &ids), Ok("a3x9k2".to_string()));
    }

    #[test]
    fn resolve_id_short_unique() {
        let ids = vec!["a3x9k2", "a3bQ7f", "m8rT2p"];
        // "m" uniquely matches "m8rT2p"
        assert_eq!(resolve_id("m", &ids), Ok("m8rT2p".to_string()));
    }

    #[test]
    fn resolve_id_short_unique_longer_prefix() {
        let ids = vec!["a3x9k2", "a3bQ7f", "m8rT2p"];
        // "a3x" uniquely matches "a3x9k2"
        assert_eq!(resolve_id("a3x", &ids), Ok("a3x9k2".to_string()));
    }

    #[test]
    fn resolve_id_ambiguous() {
        let ids = vec!["a3x9k2", "a3bQ7f", "m8rT2p"];
        // "a3" matches both "a3x9k2" and "a3bQ7f"
        let result = resolve_id("a3", &ids);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(
            err,
            IdError::Ambiguous {
                prefix: "a3".to_string(),
                matches: vec!["a3x9k2".to_string(), "a3bQ7f".to_string()]
            }
        );
    }

    #[test]
    fn resolve_id_not_found() {
        let ids = vec!["a3x9k2", "m8rT2p"];
        let result = resolve_id("zzz", &ids);
        assert_eq!(
            result,
            Err(IdError::NotFound {
                prefix: "zzz".to_string()
            })
        );
    }

    #[test]
    fn resolve_id_empty_candidates() {
        let ids: Vec<&str> = vec![];
        let result = resolve_id("a", &ids);
        assert_eq!(
            result,
            Err(IdError::NotFound {
                prefix: "a".to_string()
            })
        );
    }

    #[test]
    fn resolve_id_single_candidate() {
        let ids = vec!["only1"];
        assert_eq!(resolve_id("o", &ids), Ok("only1".to_string()));
    }

    #[test]
    fn id_error_display_not_found() {
        let err = IdError::NotFound {
            prefix: "xyz".to_string(),
        };
        assert_eq!(err.to_string(), "no match for ID prefix 'xyz'");
    }

    #[test]
    fn id_error_display_ambiguous() {
        let err = IdError::Ambiguous {
            prefix: "a3".to_string(),
            matches: vec!["a3x9k2".to_string(), "a3bQ7f".to_string()],
        };
        assert_eq!(
            err.to_string(),
            "ambiguous ID prefix 'a3' matches: a3x9k2, a3bQ7f"
        );
    }

    #[test]
    fn min_unique_prefixes_empty() {
        let ids: Vec<&str> = vec![];
        assert!(min_unique_prefixes(&ids).is_empty());
    }

    #[test]
    fn min_unique_prefixes_single() {
        let ids = vec!["abc123"];
        let result = min_unique_prefixes(&ids);
        assert_eq!(result, vec![("abc123".to_string(), 1)]);
    }

    #[test]
    fn min_unique_prefixes_distinct_first_char() {
        let ids = vec!["abc", "xyz"];
        let result = min_unique_prefixes(&ids);
        assert_eq!(result, vec![("abc".to_string(), 1), ("xyz".to_string(), 1)]);
    }

    #[test]
    fn min_unique_prefixes_shared_prefix() {
        let ids = vec!["a3x9k2", "a3bQ7f", "m8rT2p"];
        let result = min_unique_prefixes(&ids);
        // "m8rT2p" needs 1 char (m is unique first char)
        // "a3x9k2" and "a3bQ7f" share "a3", differ at 3rd char
        // so they need 3 chars each
        let map: std::collections::HashMap<String, usize> = result.into_iter().collect();
        assert_eq!(*map.get("m8rT2p").unwrap(), 1);
        assert_eq!(*map.get("a3x9k2").unwrap(), 3);
        assert_eq!(*map.get("a3bQ7f").unwrap(), 3);
    }

    #[test]
    fn min_unique_prefixes_very_similar() {
        let ids = vec!["abcdef", "abcdeg"];
        let result = min_unique_prefixes(&ids);
        let map: std::collections::HashMap<String, usize> = result.into_iter().collect();
        // They differ at position 5 (0-indexed), so need 6 chars
        assert_eq!(*map.get("abcdef").unwrap(), 6);
        assert_eq!(*map.get("abcdeg").unwrap(), 6);
    }

    #[test]
    fn min_unique_prefixes_one_is_prefix_of_other() {
        let ids = vec!["abc", "abcd"];
        let result = min_unique_prefixes(&ids);
        let map: std::collections::HashMap<String, usize> = result.into_iter().collect();
        // "abc" is a prefix of "abcd", so "abc" needs all 3 chars
        // "abcd" needs all 4 chars to distinguish from "abc"
        assert_eq!(*map.get("abc").unwrap(), 3);
        assert_eq!(*map.get("abcd").unwrap(), 4);
    }
}

//! Fuzzy subsequence match on script names. No extra crate: prefix first,
//! then the shortest window that still contains the query, then catalogue order.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuzzyMatch {
    pub index: usize,
    pub positions: Vec<usize>,
}

pub fn filter_names<'a, I>(names: I, query: &str) -> Vec<FuzzyMatch>
where
    I: IntoIterator<Item = &'a str>,
{
    let needle: Vec<char> = query.chars().collect();
    let mut scored = Vec::new();
    for (index, name) in names.into_iter().enumerate() {
        if needle.is_empty() {
            scored.push((
                0u8,
                0usize,
                index,
                FuzzyMatch {
                    index,
                    positions: Vec::new(),
                },
            ));
            continue;
        }
        let Some((prefix_rank, window, positions)) = match_name(name, &needle) else {
            continue;
        };
        scored.push((prefix_rank, window, index, FuzzyMatch { index, positions }));
    }
    scored.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    scored.into_iter().map(|(_, _, _, item)| item).collect()
}

fn chars_eq_ignore_case(left: char, right: char) -> bool {
    left == right || left.to_lowercase().eq(right.to_lowercase())
}

fn is_prefix(hay: &[char], needle: &[char]) -> bool {
    hay.len() >= needle.len()
        && needle
            .iter()
            .enumerate()
            .all(|(i, n)| chars_eq_ignore_case(hay[i], *n))
}

fn match_name(name: &str, needle: &[char]) -> Option<(u8, usize, Vec<usize>)> {
    let hay: Vec<char> = name.chars().collect();
    let mut best: Option<(usize, Vec<usize>)> = None;
    for start in 0..hay.len() {
        let mut needle_at = 0;
        let mut positions = Vec::new();
        for (hay_at, &ch) in hay.iter().enumerate().skip(start) {
            if chars_eq_ignore_case(ch, needle[needle_at]) {
                positions.push(hay_at);
                needle_at += 1;
                if needle_at == needle.len() {
                    let window = hay_at.saturating_sub(start).saturating_add(1);
                    if best
                        .as_ref()
                        .is_none_or(|(best_window, _)| window < *best_window)
                    {
                        best = Some((window, positions));
                    }
                    break;
                }
            }
        }
    }
    let (window, positions) = best?;
    let prefix_rank = if is_prefix(&hay, needle) { 0 } else { 1 };
    Some((prefix_rank, window, positions))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_keeps_declaration_order_without_highlights() {
        let got = filter_names(["dev", "build", "test"], "");
        assert_eq!(
            got,
            vec![
                FuzzyMatch {
                    index: 0,
                    positions: vec![]
                },
                FuzzyMatch {
                    index: 1,
                    positions: vec![]
                },
                FuzzyMatch {
                    index: 2,
                    positions: vec![]
                },
            ]
        );
    }

    #[test]
    fn subsequence_is_case_insensitive() {
        let got = filter_names(["DevServer", "other"], "dvs");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].index, 0);
        assert_eq!(got[0].positions, vec![0, 2, 3]);
    }

    #[test]
    fn prefix_ranks_ahead_of_a_looser_subsequence() {
        let got = filter_names(["latest", "test"], "te");
        assert_eq!(
            got.iter().map(|item| item.index).collect::<Vec<_>>(),
            vec![1, 0]
        );
    }

    #[test]
    fn compact_window_beats_a_wider_subsequence() {
        let got = filter_names(["xa--bc", "xabc"], "abc");
        assert_eq!(got[0].index, 1);
        assert_eq!(got[1].index, 0);
    }

    #[test]
    fn ties_keep_declaration_order() {
        let got = filter_names(["alpha", "alpine"], "al");
        assert_eq!(
            got.iter().map(|item| item.index).collect::<Vec<_>>(),
            vec![0, 1]
        );
    }

    #[test]
    fn unicode_positions_are_character_indexes() {
        let got = filter_names(["日本語テスト", "ascii"], "本テ");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].positions, vec![1, 3]);
        assert_eq!("日本語テスト".chars().count(), 6);
    }

    #[test]
    fn missing_character_is_not_a_match() {
        assert!(filter_names(["dev", "build"], "xyz").is_empty());
    }
}

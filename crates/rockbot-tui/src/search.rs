use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config, Matcher, Utf32Str,
};

pub fn fuzzy_indices<I>(query: &str, items: I) -> Vec<usize>
where
    I: IntoIterator<Item = (usize, String)>,
{
    let items: Vec<(usize, String)> = items.into_iter().collect();
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return items.into_iter().map(|(idx, _)| idx).collect();
    }

    let pattern = Pattern::parse(trimmed, CaseMatching::Smart, Normalization::Smart);
    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut matches = Vec::new();
    let mut buf = Vec::new();

    for (idx, haystack) in items {
        if let Some(score) = pattern.score(Utf32Str::new(haystack.as_str(), &mut buf), &mut matcher)
        {
            matches.push((idx, score));
        }
    }

    matches.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    matches.into_iter().map(|(idx, _)| idx).collect()
}

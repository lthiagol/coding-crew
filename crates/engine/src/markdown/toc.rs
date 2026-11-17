#[derive(Clone)]
pub struct TocEntry {
    pub level: u8,
    pub title: String,
    pub line: usize,
}

fn distinct_levels(toc: &[TocEntry]) -> Vec<u8> {
    let mut levels: Vec<u8> = toc.iter().map(|e| e.level).collect();
    levels.sort_unstable();
    levels.dedup();
    levels
}

pub(crate) fn normalize_toc(mut toc: Vec<TocEntry>) -> Vec<TocEntry> {
    let levels = distinct_levels(&toc);
    let Some(&max_keep) = levels.get(2).or_else(|| levels.last()) else {
        return toc;
    };
    toc.retain(|entry| entry.level <= max_keep);
    toc
}

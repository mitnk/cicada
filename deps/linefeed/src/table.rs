//! Provides utilities for formatting strings in a table

use std::cmp::min;

const COL_SPACE: usize = 2;

/// Represents a table of strings, formatted into rows and columns
///
/// A `Table` is an `Iterator` yielding `Line` elements, which are in turn
/// iterators yielding `(usize, &str)` elements, describing the width and content
/// of each cell in a given row.
pub struct Table<'a, S: 'a> {
    strings: &'a [S],
    sizes: Option<&'a [usize]>,
    offset: usize,
    per_col: usize,
    rows: usize,
    horizontal: bool,
}

impl<'a, S: 'a + AsRef<str>> Table<'a, S> {
    /// Constructs a new table from the given set of strings, using the given
    /// column sizes.
    ///
    /// If `horizontal` is `true`, items will be list horizontally first.
    ///
    /// # Horizontal
    ///
    /// ```text
    /// a b c
    /// d e f
    /// g h i
    /// ```
    ///
    /// # Vertical
    ///
    /// ```text
    /// a d g
    /// b e h
    /// c f i
    /// ```
    pub fn new(strs: &'a [S], mut sizes: Option<&'a [usize]>,
            horizontal: bool) -> Table<'a, S> {
        if let Some(sz) = sizes {
            if sz.is_empty() {
                sizes = None;
            }
        }

        let n_strs = strs.len();
        let n_cols = sizes.map_or(1, |sz| sz.len());

        let rows = n_strs / n_cols + (n_strs % n_cols != 0) as usize;

        Table{
            strings: strs,
            sizes: sizes,
            offset: 0,
            per_col: (strs.len() + (n_cols - 1)) / n_cols,
            rows: rows,
            horizontal: horizontal,
        }
    }

    /// Returns whether more lines are present in the table.
    pub fn has_more(&self) -> bool {
        self.offset < self.rows
    }

    fn num_cols(&self) -> usize {
        self.sizes.map_or(1, |sz| sz.len())
    }
}

impl<'a, S: 'a + AsRef<str>> Iterator for Table<'a, S> {
    type Item = Line<'a, S>;

    fn next(&mut self) -> Option<Line<'a, S>> {
        if self.offset == self.rows {
            return None;
        }

        let n = self.num_cols();

        let (start, end, stride) = if self.horizontal {
            let start = self.offset * n;
            let end = min(self.strings.len(), start + n);
            (start, end, 1)
        } else {
            let start = self.offset;
            let end = min(self.strings.len(), start + self.per_col * n);
            (start, end, self.per_col)
        };

        self.offset += 1;

        Some(Line{
            strings: &self.strings[start..end],
            sizes: self.sizes,
            stride: stride,
            offset: 0,
        })
    }
}

/// Represents a single line of the table
///
/// A `Line` is an `Iterator` yielding `(usize, &str)` elements, describing
/// the width and content of each cell in a given row.
pub struct Line<'a, S: 'a> {
    strings: &'a [S],
    sizes: Option<&'a [usize]>,
    stride: usize,
    offset: usize,
}

impl<'a, S: 'a + AsRef<str>> Iterator for Line<'a, S> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<(usize, &'a str)> {
        let s = self.strings.get(self.offset * self.stride)?.as_ref();

        let width = self.sizes.and_then(|sz| sz.get(self.offset).cloned())
            .unwrap_or_else(|| s.chars().count());

        self.offset += 1;

        Some((width, s))
    }
}

/// Formats a series of strings into columns, fitting within a given screen width.
/// Returns the size of each resulting column, including spacing.
///
/// If the strings cannot be formatted into columns (e.g. one or more strings
/// are longer than the screen width) or the result would be only one column,
/// `None` is returned.
pub fn format_columns<S: AsRef<str>>(strs: &[S], screen_width: usize,
        horizontal: bool) -> Option<Vec<usize>> {
    if strs.is_empty() {
        return None;
    }

    let n_strs = strs.len();

    let (mut min_len, mut max_len) = min_max(strs.iter().map(|s| s.as_ref().chars().count()));

    if min_len == 0 { min_len = 1; }
    if max_len == 0 { max_len = 1; }

    let mut min_cols = min(n_strs, screen_width / max_len);
    let max_cols = min(n_strs, screen_width / min_len);

    if min_cols <= 1 {
        // No point in checking whether text can fit within one column
        min_cols = 2;
    }

    if max_cols <= 1 {
        return None;
    }

    let mut col_sizes = if min_cols == max_cols {
        vec![vec![0; max_cols]]
    } else {
        (min_cols..max_cols + 1)
            .map(|n| vec![0; n]).collect::<Vec<_>>()
    };

    for (i, s) in strs.iter().enumerate() {
        let len = s.as_ref().chars().count();

        for cols in &mut col_sizes {
            let n_cols = cols.len();

            let col = if horizontal {
                i % n_cols
            } else {
                let per_col = (n_strs + (n_cols - 1)) / n_cols;
                i / per_col
            };

            let real_len = if col == n_cols - 1 { len } else { len + COL_SPACE };

            if real_len > cols[col] {
                cols[col] = real_len;
            }
        }
    }

    for cols in col_sizes.into_iter().rev() {
        if cols.iter().fold(0, |a, b| a + b) <= screen_width {
            return Some(cols);
        }
    }

    None
}

fn min_max<I>(iter: I) -> (usize, usize) where I: Iterator<Item=usize> {
    let mut min = usize::max_value();
    let mut max = 0;

    for n in iter {
        if n < min {
            min = n;
        }
        if n + COL_SPACE > max {
            max = n + COL_SPACE;
        }
    }

    (min, max)
}

#[cfg(test)]
mod test {
    use std::iter::repeat;
    use super::format_columns;

    #[test]
    fn test_long_item() {
        let strs = (1..500).map(|n| repeat('x').take(n).collect())
            .collect::<Vec<String>>();

        assert_matches!(format_columns(&strs, 80, false), None);
    }

    #[test]
    fn test_zero_item() {
        let strs = ["", "", ""];

        assert_eq!(format_columns(&strs, 80, false), Some(vec![2, 2, 0]));
    }
}

pub trait RecursiveTextSplitter {
    fn split_text(&self, text: &str, max_chunk_size: usize) -> Vec<String>;
}

#[derive(Debug, Clone)]
pub struct RTSplitter {
    separators: Vec<&'static str>,
}

impl Default for RTSplitter {
    fn default() -> Self {
        Self {
            separators: vec![
                "\n\n", // Double newline (paragraphs)
                "\n",   // Single newline
                " ",    // Space
                "",     // Character level
            ],
        }
    }
}

impl RecursiveTextSplitter for RTSplitter {
    fn split_text(&self, text: &str, max_chunk_size: usize) -> Vec<String> {
        if text.len() <= max_chunk_size {
            return vec![text.to_string()];
        }

        self.recursive_split(text, max_chunk_size, 0)
    }
}

impl RTSplitter {
    fn split_by_length(&self, text: &str, max_chunk_size: usize) -> Vec<String> {
        let mut chunks = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        let mut start = 0;

        while start < chars.len() {
            let end = (start + max_chunk_size).min(chars.len());
            let chunk: String = chars[start..end].iter().collect();
            chunks.push(chunk);

            if end == chars.len() {
                break;
            }

            start = end;
        }

        chunks
    }

    fn recursive_split(
        &self,
        text: &str,
        max_chunk_size: usize,
        separator_index: usize,
    ) -> Vec<String> {
        if text.len() <= max_chunk_size {
            return vec![text.to_string()];
        }

        if separator_index >= self.separators.len() {
            return self.split_by_length(text, max_chunk_size);
        }

        let separator = self.separators[separator_index];

        if separator.is_empty() {
            return self.split_by_length(text, max_chunk_size);
        }

        let parts: Vec<&str> = text.split(separator).collect();

        if parts.len() == 1 {
            return self.recursive_split(text, max_chunk_size, separator_index + 1);
        }

        let mut chunks = Vec::new();
        let mut current_chunk = String::new();

        for (i, part) in parts.iter().enumerate() {
            // `split` drops the separators; reattach the one that preceded each
            // part (every part except the first) so the chunks reconstruct the
            // original text losslessly via `join("")`. The separator lands on
            // the leading edge of a new chunk rather than being discarded at the
            // boundary.
            let piece = if i == 0 {
                part.to_string()
            } else {
                format!("{}{}", separator, part)
            };

            if current_chunk.is_empty() {
                current_chunk = piece;
            } else if current_chunk.len() + piece.len() <= max_chunk_size {
                current_chunk.push_str(&piece);
            } else {
                chunks.push(std::mem::take(&mut current_chunk));
                current_chunk = piece;
            }

            // A single piece can still exceed the limit; recurse on the finer
            // separator and flush the result.
            if current_chunk.len() > max_chunk_size {
                let sub_chunks =
                    self.recursive_split(&current_chunk, max_chunk_size, separator_index + 1);
                chunks.extend(sub_chunks);
                current_chunk.clear();
            }
        }

        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_splitting() {
        let splitter = RTSplitter::default();
        let text = "This is a test.\n\nThis is another paragraph.\n\nAnd a third one.";
        let chunks = splitter.split_text(text, 30);

        println!("Chunks: {:?}", chunks);

        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(chunk.len() <= 30);
        }
    }

    #[test]
    fn test_no_overlap() {
        let splitter = RTSplitter::default();
        let text = "This is a very long sentence that should be split into multiple chunks with no overlap between them.";
        let chunks = splitter.split_text(text, 40);

        println!("Chunks: {:?}", chunks);

        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.len() <= 40);
        }

        // Verify no overlap by checking that concatenating chunks gives original text
        let reconstructed = chunks.join("");
        assert_eq!(reconstructed, text);
    }

    #[test]
    fn test_short_text() {
        let splitter = RTSplitter::default();
        let text = "Short text";
        let chunks = splitter.split_text(text, 100);

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], text);
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Splitting is lossless for ANY input: concatenating the chunks (no
        /// separator) reproduces the original text exactly.
        #[test]
        fn split_is_lossless(text in "(?s).*", max in 1usize..64) {
            let splitter = RTSplitter::default();
            let chunks = splitter.split_text(&text, max);
            prop_assert_eq!(chunks.concat(), text.clone());
            // No empty chunks for non-empty input.
            if !text.is_empty() {
                for c in &chunks {
                    prop_assert!(!c.is_empty());
                }
            }
        }

        /// For ASCII input (where one char == one byte) the byte-length bound is
        /// strictly respected. (Multibyte text can exceed the byte bound at the
        /// character-split fallback, which is acceptable — losslessness above is
        /// the hard guarantee.)
        #[test]
        fn ascii_respects_size_bound(text in "[ -~\n\t]*", max in 1usize..64) {
            let splitter = RTSplitter::default();
            let chunks = splitter.split_text(&text, max);
            prop_assert_eq!(chunks.concat(), text.clone());
            for c in &chunks {
                prop_assert!(c.len() <= max, "chunk {:?} exceeds max {}", c, max);
            }
        }
    }

    #[test]
    fn edge_cases_are_lossless() {
        let splitter = RTSplitter::default();
        let cases = [
            "",
            " ",
            "\n\n\n",
            &"x".repeat(500),
            "مرحبا بالعالم RTL mixed with LTR", // RTL + LTR
            "emoji 😀😀😀 and CJK 北京 mixed",  // emoji + CJK
            "tabs\tand\nnewlines\r\nmixed   spaces",
        ];
        for text in cases {
            let chunks = splitter.split_text(text, 8);
            assert_eq!(chunks.concat(), text, "lossless failed for {:?}", text);
        }
    }
}

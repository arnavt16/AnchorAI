use anchor_lib::indexing::chunker::{chunk_text, TARGET_MAX_WORDS};

#[test]
fn short_text_stays_one_chunk() {
    let text = "Just a short thought about today.";
    let chunks = chunk_text(text);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], text);
}

#[test]
fn empty_text_produces_no_chunks() {
    assert!(chunk_text("   ").is_empty());
    assert!(chunk_text("").is_empty());
}

#[test]
fn long_text_splits_on_paragraph_boundaries() {
    let para = "word ".repeat(150);
    let text = format!("{para}\n\n{para}\n\n{para}");
    let chunks = chunk_text(&text);
    assert!(chunks.len() >= 2, "450 words across 3 paragraphs should split into at least 2 chunks");
    for c in &chunks {
        let words = c.split_whitespace().count();
        assert!(words <= TARGET_MAX_WORDS + 5, "chunk exceeded target word budget: {words}");
    }
}

#[test]
fn oversized_single_paragraph_is_hard_wrapped_not_truncated() {
    let text = "word ".repeat(1000); // one giant paragraph, no blank lines
    let chunks = chunk_text(&text);
    let total_words: usize = chunks.iter().map(|c| c.split_whitespace().count()).sum();
    assert_eq!(total_words, 1000, "hard-wrapping must not silently drop words");
    assert!(chunks.len() > 1);
}

#[test]
fn chunking_never_drops_content() {
    let para = "alpha ".repeat(80);
    let text = format!("{para}\n\n{}", "beta ".repeat(80));
    let chunks = chunk_text(&text);
    let rejoined_word_count: usize = chunks.iter().map(|c| c.split_whitespace().count()).sum();
    let original_word_count = text.split_whitespace().count();
    assert_eq!(rejoined_word_count, original_word_count);
}

const CHUNK_SIZE: usize = 800;
const CHUNK_OVERLAP: usize = 100;
const MIN_CHUNK_SIZE: usize = 50;

pub struct Chunk {
    pub content: String,
    pub chunk_index: usize,
}

/// Snap a byte offset to the nearest char boundary (forward).
fn snap_forward(s: &str, mut pos: usize) -> usize {
    while pos < s.len() && !s.is_char_boundary(pos) {
        pos += 1;
    }
    pos
}

/// Snap a byte offset to the nearest char boundary (backward).
fn snap_backward(s: &str, mut pos: usize) -> usize {
    while pos > 0 && !s.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
}

/// Split content into drawer-sized chunks, breaking at paragraph/line boundaries.
pub fn chunk_text(content: &str) -> Vec<Chunk> {
    let content = content.trim();
    if content.is_empty() {
        return vec![];
    }

    let mut chunks = Vec::new();
    let mut start = 0;
    let mut chunk_index = 0;

    while start < content.len() {
        let mut end = snap_backward(content, (start + CHUNK_SIZE).min(content.len()));

        // Try to break at paragraph boundary, then line boundary
        if end < content.len() {
            if let Some(pos) = content[start..end].rfind("\n\n") {
                let abs_pos = start + pos;
                if abs_pos > start + CHUNK_SIZE / 2 {
                    end = abs_pos;
                }
            } else if let Some(pos) = content[start..end].rfind('\n') {
                let abs_pos = start + pos;
                if abs_pos > start + CHUNK_SIZE / 2 {
                    end = abs_pos;
                }
            }
        }

        let chunk = content[start..end].trim();
        if chunk.len() >= MIN_CHUNK_SIZE {
            chunks.push(Chunk {
                content: chunk.to_string(),
                chunk_index,
            });
            chunk_index += 1;
        }

        if end >= content.len() {
            break;
        }
        start = snap_forward(content, end.saturating_sub(CHUNK_OVERLAP));
    }

    chunks
}

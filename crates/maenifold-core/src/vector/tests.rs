use super::*;

fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    let dot = left
        .iter()
        .zip(right.iter())
        .map(|(left, right)| left * right)
        .sum::<f32>();
    let left_mag = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    let right_mag = right.iter().map(|value| value * value).sum::<f32>().sqrt();
    if left_mag == 0.0 || right_mag == 0.0 {
        return 0.0;
    }
    dot / (left_mag * right_mag)
}

#[test]
fn vector_generate_embedding_hello_world() {
    cleanup();
    load_model().expect("model loads");
    let embedding = generate_embedding("hello world").expect("embedding generated");
    assert_eq!(
        embedding.len(),
        embedding_dimensions().unwrap_or(DEFAULT_EMBEDDING_DIMENSIONS)
    );
    assert!(
        embedding.iter().any(|value| *value != 0.0),
        "hello world embedding should not be all zeros"
    );
    let self_similarity = cosine_similarity(&embedding, &embedding);
    assert!(
        (self_similarity - 1.0).abs() < 1e-5,
        "self cosine similarity should be ~1.0, got {self_similarity}"
    );
}

#[test]
fn vector_batch_matches_individual() {
    cleanup();
    let text = "hello world";
    let individual = generate_embedding(text).expect("individual embedding");
    let batched = generate_embeddings(&[text], DEFAULT_MAX_BATCH).expect("batch embedding");
    assert_eq!(batched.len(), 1);
    assert_eq!(
        batched[0].len(),
        embedding_dimensions().unwrap_or(DEFAULT_EMBEDDING_DIMENSIONS)
    );
    for (index, (left, right)) in individual.iter().zip(batched[0].iter()).enumerate() {
        assert!(
            (left - right).abs() <= 1e-6,
            "batch result diverged at dimension {index}: {left} vs {right}"
        );
    }
}

#[test]
fn vector_base64_round_trip_preserves_embedding() {
    cleanup();
    let embedding = generate_embedding("hello world").expect("embedding generated");
    let encoded = generate_embedding_base64("hello world").expect("base64 generated");
    let decoded = decode_base64_embedding(&encoded);
    assert_eq!(decoded.len(), embedding.len());
    for (index, (left, right)) in embedding.iter().zip(decoded.iter()).enumerate() {
        assert!(
            (left - right).abs() <= f32::EPSILON,
            "base64 round-trip diverged at dimension {index}: {left} vs {right}"
        );
    }
}

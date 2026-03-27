pub fn assert_framebuffer_eq(actual: &[u8], expected: &[u8], context: &str) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "{context}: framebuffer lengths differ (actual={} expected={})",
        actual.len(),
        expected.len()
    );

    if actual == expected {
        return;
    }

    let mut mismatches = Vec::new();
    for (index, (actual_value, expected_value)) in actual.iter().zip(expected.iter()).enumerate() {
        if actual_value != expected_value {
            mismatches.push(format!(
                "#{index}: {actual_value:02X} != {expected_value:02X}"
            ));
            if mismatches.len() == 8 {
                break;
            }
        }
    }

    panic!(
        "{context}: framebuffer mismatch; first differences: {}",
        mismatches.join(", ")
    );
}

pub fn assert_framebuffer_ne(actual: &[u8], expected: &[u8], context: &str) {
    assert_ne!(
        actual, expected,
        "{context}: expected framebuffer contents to differ"
    );
}

pub fn assert_audio_samples_eq(actual: &[f32], expected: &[f32], epsilon: f32, context: &str) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "{context}: sample lengths differ (actual={} expected={})",
        actual.len(),
        expected.len()
    );

    let mut mismatches = Vec::new();
    for (index, (actual_value, expected_value)) in actual.iter().zip(expected.iter()).enumerate() {
        if (actual_value - expected_value).abs() > epsilon {
            mismatches.push(format!(
                "#{index}: {:.6} != {:.6}",
                actual_value, expected_value
            ));
            if mismatches.len() == 8 {
                break;
            }
        }
    }

    if mismatches.is_empty() {
        return;
    }

    panic!(
        "{context}: audio samples diverged beyond epsilon {epsilon}; first differences: {}",
        mismatches.join(", ")
    );
}

pub fn assert_audio_has_activity(samples: &[f32], minimum_amplitude: f32, context: &str) {
    assert!(
        samples
            .iter()
            .any(|sample| sample.abs() >= minimum_amplitude),
        "{context}: expected audible activity with amplitude >= {minimum_amplitude}, got {samples:?}"
    );
}

pub fn assert_audio_silent(samples: &[f32], epsilon: f32, context: &str) {
    assert!(
        samples.iter().all(|sample| sample.abs() <= epsilon),
        "{context}: expected silence within epsilon {epsilon}, got {samples:?}"
    );
}

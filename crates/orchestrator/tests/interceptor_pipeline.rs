use actrpc_orchestrator::interceptor::{ImmutableInterceptorPipeline, WorkingInterceptorPipeline};

#[test]
fn immutable_pipeline_exposes_order() {
    let pipeline =
        ImmutableInterceptorPipeline::new(vec!["b".to_owned(), "a".to_owned(), "c".to_owned()]);

    assert_eq!(pipeline.len(), 3);
    assert!(!pipeline.is_empty());
    assert_eq!(pipeline.as_slice(), ["b", "a", "c"]);
}

#[test]
fn immutable_pipeline_snapshot_is_independent_working_copy() {
    let immutable = ImmutableInterceptorPipeline::new(vec![
        "first".to_owned(),
        "second".to_owned(),
        "third".to_owned(),
    ]);

    let working = immutable.snapshot();

    working.exclude_named(&["second".to_owned()]);

    assert_eq!(working.snapshot(), ["first", "third"]);
    assert_eq!(immutable.as_slice(), ["first", "second", "third"]);
}

#[test]
fn working_pipeline_excludes_named_interceptors() {
    let pipeline = WorkingInterceptorPipeline::new(vec![
        "a".to_owned(),
        "b".to_owned(),
        "c".to_owned(),
        "b".to_owned(),
    ]);

    pipeline.exclude_named(&["b".to_owned(), "missing".to_owned()]);

    assert_eq!(pipeline.snapshot(), ["a", "c"]);
}

#[test]
fn working_pipeline_empty_exclusion_is_noop() {
    let pipeline = WorkingInterceptorPipeline::new(vec!["a".to_owned(), "b".to_owned()]);

    pipeline.exclude_named(&[]);

    assert_eq!(pipeline.snapshot(), ["a", "b"]);
}

#[test]
fn immutable_pipeline_creates_working_pipeline_snapshot() {
    let immutable = ImmutableInterceptorPipeline::new(vec![
        "first".to_owned(),
        "second".to_owned(),
        "third".to_owned(),
    ]);

    let working = immutable.snapshot();

    assert_eq!(working.snapshot(), ["first", "second", "third"]);

    working.exclude_named(&["second".to_owned()]);

    assert_eq!(working.snapshot(), ["first", "third"]);
    assert_eq!(immutable.as_slice(), ["first", "second", "third"]);
}

use std::sync::RwLock;

#[derive(Debug, Clone, Default)]
pub struct ImmutableInterceptorPipeline {
    interceptors: Vec<String>,
}

impl ImmutableInterceptorPipeline {
    pub fn new(interceptors: Vec<String>) -> Self {
        Self { interceptors }
    }

    pub fn snapshot(&self) -> WorkingInterceptorPipeline {
        WorkingInterceptorPipeline::new(self.interceptors.clone())
    }

    pub fn as_slice(&self) -> &[String] {
        &self.interceptors
    }

    pub fn is_empty(&self) -> bool {
        self.interceptors.is_empty()
    }

    pub fn len(&self) -> usize {
        self.interceptors.len()
    }
}

#[derive(Debug, Default)]
pub struct WorkingInterceptorPipeline {
    interceptors: RwLock<Vec<String>>,
}

impl WorkingInterceptorPipeline {
    pub fn new(interceptors: Vec<String>) -> Self {
        Self {
            interceptors: RwLock::new(interceptors),
        }
    }

    pub fn snapshot(&self) -> Vec<String> {
        self.interceptors
            .read()
            .expect("poisoned working pipeline lock")
            .clone()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.interceptors
            .read()
            .expect("poisoned working pipeline lock")
            .iter()
            .any(|current| current == name)
    }

    pub fn exclude_named(&self, names: &[String]) {
        if names.is_empty() {
            return;
        }

        let mut interceptors = self
            .interceptors
            .write()
            .expect("poisoned working pipeline lock");

        interceptors.retain(|current| !names.iter().any(|name| name == current));
    }
}
